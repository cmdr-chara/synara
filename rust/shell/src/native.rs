//! Native gpui-kit workspace: sidebar (spaces/projects/threads) + transcript +
//! composer, fed by rust/app's native WS-RPC (getShellSnapshot /
//! getThreadDetailSnapshot / dispatchCommand relayed upstream).

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;

use crate::ws_client::Client;

actions!(native, [ToggleFind, ToggleTerminal]);

// Real upstream icon assets: Hugeicons glyphs regenerated as SVG, Central
// icons copied verbatim from apps/web/public/central-icons-reversed.
mod icons {
    pub const HOME: &[u8] = include_bytes!("../assets/icons/home-07.svg");
    pub const INBOX: &[u8] = include_bytes!("../assets/icons/inbox.svg");
    pub const SPACES: &[u8] = include_bytes!("../assets/icons/folder-library.svg");
    pub const KANBAN: &[u8] = include_bytes!("../assets/icons/columns-3-wide.svg");
    pub const TASKS: &[u8] = include_bytes!("../assets/icons/apple-reminder.svg");
    pub const PULL_REQUESTS: &[u8] =
        include_bytes!("../assets/icons/git-compare-arrows.svg");
    pub const AUTOMATIONS: &[u8] = include_bytes!("../assets/icons/clock-hour-7.svg");
    pub const STUDIO: &[u8] = include_bytes!("../assets/icons/dashboard-circle.svg");
    pub const SIDEBAR_LEFT: &[u8] =
        include_bytes!("../assets/icons/sidebar-simple-left-wide.svg");
    pub const SIDEBAR_RIGHT: &[u8] =
        include_bytes!("../assets/icons/sidebar-simple-right-wide.svg");
    pub const SEARCH: &[u8] = include_bytes!("../assets/icons/magnifying-glass.svg");
    pub const NEW_THREAD: &[u8] = include_bytes!("../assets/icons/compose-pencil.svg");
    pub const BELL: &[u8] = include_bytes!("../assets/icons/notes.svg");
    pub const USER: &[u8] = include_bytes!("../assets/icons/user.svg");
    pub const HELP: &[u8] = include_bytes!("../assets/icons/circle-questionmark.svg");
    pub const MORE: &[u8] = include_bytes!("../assets/icons/dot-grid-1x3-horizontal.svg");
    pub const PLUS: &[u8] = include_bytes!("../assets/icons/plus-sign.svg");
    pub const SEND: &[u8] = include_bytes!("../assets/icons/arrow-up.svg");
    pub const ARROW_DOWN: &[u8] = include_bytes!("../assets/icons/arrow-down.svg");
    pub const X: &[u8] = include_bytes!("../assets/icons/x.svg");
    pub const UNDO: &[u8] = include_bytes!("../assets/icons/arrow-undo-up.svg");
    pub const FILE_TEXT: &[u8] = include_bytes!("../assets/icons/file-text.svg");
    pub const PAPERCLIP: &[u8] = include_bytes!("../assets/icons/paperclip-1.svg");
    pub const WINDOW: &[u8] = include_bytes!("../assets/icons/window.svg");
    pub const GOAL: &[u8] = include_bytes!("../assets/icons/target-arrow.svg");
    pub const TODOS: &[u8] = include_bytes!("../assets/icons/todos.svg");
    pub const BUG: &[u8] = include_bytes!("../assets/icons/bug.svg");
    pub const ZAP: &[u8] = include_bytes!("../assets/icons/zap.svg");
    pub const FLAG: &[u8] = include_bytes!("../assets/icons/flag.svg");
    pub const CHECK: &[u8] = include_bytes!("../assets/icons/checkmark-1.svg");
    pub const UNSTAGE: &[u8] =
        include_bytes!("../assets/icons/arrow-rotate-counter-clockwise.svg");
    pub const STAGE: &[u8] = include_bytes!("../assets/icons/plus-medium.svg");
    pub const FOLDER: &[u8] = include_bytes!("../assets/icons/folder-closed.svg");
    pub const TERMINAL: &[u8] = include_bytes!("../assets/icons/square-terminal.svg");
    pub const PIN: &[u8] = include_bytes!("../assets/icons/pin.svg");
    pub const PANEL_TOP_OPEN: &[u8] =
        include_bytes!("../assets/icons/panel-top-open.svg");
    pub const LAYOUT_RIGHT: &[u8] = include_bytes!("../assets/icons/layout-right.svg");
    pub const PIN_FILL: &[u8] = include_bytes!("../assets/icons/pin-fill.svg");
    pub const STAR: &[u8] = include_bytes!("../assets/icons/star.svg");
    pub const FOLDER_OPEN: &[u8] = include_bytes!("../assets/icons/folder-open.svg");
    pub const CODEX: &[u8] = include_bytes!("../assets/icons/codex.svg");
    pub const P_CLAUDE: &[u8] = include_bytes!("../assets/icons/claude.svg");
    pub const P_CURSOR: &[u8] = include_bytes!("../assets/icons/cursor.svg");
    pub const P_GROK: &[u8] = include_bytes!("../assets/icons/grok.svg");
    pub const P_PI: &[u8] = include_bytes!("../assets/icons/pi.svg");
    pub const P_OPENCODE: &[u8] = include_bytes!("../assets/icons/opencode.svg");
    pub const P_DROID: &[u8] = include_bytes!("../assets/icons/droid.svg");
    pub const P_DEVIN: &[u8] = include_bytes!("../assets/icons/devin.svg");
    pub const P_OMP: &[u8] = include_bytes!("../assets/icons/omp.svg");
    pub const P_ANTIGRAVITY: &[u8] = include_bytes!("../assets/icons/antigravity.svg");
    pub const LOGO: &[u8] = include_bytes!("../assets/icons/synara-logo.svg");
    pub const SHIELD_ACCESS: &[u8] =
        include_bytes!("../assets/icons/shield-access.svg");
    pub const SHIELD_CODE: &[u8] = include_bytes!("../assets/icons/shield-code.svg");
    pub const CHEVRON_DOWN: &[u8] =
        include_bytes!("../assets/icons/chevron-down-small.svg");
    pub const CHEVRON_RIGHT: &[u8] =
        include_bytes!("../assets/icons/chevron-right.svg");
    pub const WORKTREE: &[u8] =
        include_bytes!("../assets/icons/worktree-split.svg");
    pub const REFRESH: &[u8] =
        include_bytes!("../assets/icons/arrow-rotate-clockwise.svg");
    pub const SORT: &[u8] =
        include_bytes!("../assets/icons/sort-arrow-up-down.svg");
    pub const FILTER: &[u8] = include_bytes!("../assets/icons/filter-1.svg");
    pub const PLUS_MED: &[u8] = include_bytes!("../assets/icons/plus-medium.svg");
    pub const PENCIL: &[u8] = include_bytes!("../assets/icons/pencil.svg");
    pub const TRASH: &[u8] = include_bytes!("../assets/icons/trash-can.svg");
    pub const S_PEOPLE_ID_CARD: &[u8] =
        include_bytes!("../assets/icons/people-id-card.svg");
    pub const S_COLOR_PALETTE: &[u8] =
        include_bytes!("../assets/icons/color-palette.svg");
    pub const S_LAYOUT_SIDEBAR: &[u8] =
        include_bytes!("../assets/icons/layout-sidebar.svg");
    pub const S_CHAT_BUBBLES: &[u8] =
        include_bytes!("../assets/icons/chat-bubbles.svg");
    pub const S_BELL: &[u8] = include_bytes!("../assets/icons/bell.svg");
    pub const S_SHORTCUT: &[u8] = include_bytes!("../assets/icons/shortcut.svg");
    pub const S_PUZZLE: &[u8] = include_bytes!("../assets/icons/puzzle.svg");
    pub const S_BRAIN: &[u8] = include_bytes!("../assets/icons/brain.svg");
    pub const S_BUILDING_BLOCKS: &[u8] =
        include_bytes!("../assets/icons/building-blocks.svg");
    pub const S_GAUGE: &[u8] = include_bytes!("../assets/icons/gauge.svg");
    pub const S_CONNECTORS: &[u8] =
        include_bytes!("../assets/icons/connectors-1.svg");
    pub const S_PLUGIN: &[u8] = include_bytes!("../assets/icons/plugin-1.svg");
    pub const S_GLOBE: &[u8] = include_bytes!("../assets/icons/globe.svg");
    pub const S_COMPUTER_USE: &[u8] =
        include_bytes!("../assets/icons/computer-use.svg");
    pub const S_SCREEN_CAPTURE: &[u8] =
        include_bytes!("../assets/icons/screen-capture.svg");
    pub const S_BRANCH_SIMPLE: &[u8] =
        include_bytes!("../assets/icons/branch-simple.svg");
    pub const S_ARCHIVE: &[u8] = include_bytes!("../assets/icons/archive.svg");
    pub const S_SHIELD: &[u8] = include_bytes!("../assets/icons/shield.svg");
    pub const HANDOFF: &[u8] = include_bytes!("../assets/icons/handoff.svg");
    pub const COPY: &[u8] = include_bytes!("../assets/icons/copy.svg");
    pub const TEXT_WRAP: &[u8] = include_bytes!("../assets/icons/text-wrap.svg");
    pub const BRANCH_SIMPLE: &[u8] = include_bytes!("../assets/icons/branch-simple.svg");
    pub const GIT_COMMIT: &[u8] = include_bytes!("../assets/icons/commits.svg");
    pub const ROBOT: &[u8] = include_bytes!("../assets/icons/robot-3.svg");
    pub const DOWN_WALL: &[u8] = include_bytes!("../assets/icons/arrow-down-wall.svg");
    pub const SIDECHAT: &[u8] = include_bytes!("../assets/icons/sidechat.svg");
    pub const CLOCK: &[u8] = include_bytes!("../assets/icons/clock.svg");
    pub const S_TOOLBOX: &[u8] = include_bytes!("../assets/icons/toolbox.svg");
    pub const S_CIRCLE_INFO: &[u8] =
        include_bytes!("../assets/icons/circle-info.svg");
    pub const S_GEAR: &[u8] =
        include_bytes!("../assets/icons/settings-gear-4.svg");
    pub const B_SUPABASE: &[u8] = include_bytes!("../assets/icons/brands/supabase.svg");
    pub const B_VERCEL: &[u8] = include_bytes!("../assets/icons/brands/vercel.svg");
    pub const B_SLACK: &[u8] = include_bytes!("../assets/icons/brands/slack.svg");
    pub const B_LINEAR: &[u8] = include_bytes!("../assets/icons/brands/linear.svg");
    pub const B_NOTION: &[u8] = include_bytes!("../assets/icons/brands/notion.svg");
    pub const B_STRIPE: &[u8] = include_bytes!("../assets/icons/brands/stripe.svg");
    pub const B_WORKOS: &[u8] = include_bytes!("../assets/icons/brands/workos.svg");
    pub const B_CONVEX: &[u8] = include_bytes!("../assets/icons/brands/convex.svg");
    pub const B_CLOUDFLARE: &[u8] = include_bytes!("../assets/icons/brands/cloudflare.svg");
    pub const B_SENTRY: &[u8] = include_bytes!("../assets/icons/brands/sentry.svg");
    pub const B_NEON: &[u8] = include_bytes!("../assets/icons/brands/neon.svg");
    pub const B_POSTHOG: &[u8] = include_bytes!("../assets/icons/brands/posthog.svg");
    pub const B_NETLIFY: &[u8] = include_bytes!("../assets/icons/brands/netlify.svg");
    pub const B_RAILWAY: &[u8] = include_bytes!("../assets/icons/brands/railway.svg");
    pub const B_PRISMA: &[u8] = include_bytes!("../assets/icons/brands/prisma.svg");
    pub const B_HUGGINGFACE: &[u8] = include_bytes!("../assets/icons/brands/huggingface.svg");
    pub const B_ATLASSIAN: &[u8] = include_bytes!("../assets/icons/brands/atlassian.svg");
    pub const B_TODOIST: &[u8] = include_bytes!("../assets/icons/brands/todoist.svg");
    pub const B_MCP: &[u8] = include_bytes!("../assets/icons/brands/mcp.svg");
}

fn svg_icon(data: &'static [u8]) -> Icon {
    Icon::default().data(data)
}

// ---- keyboard-shortcut helpers (Settings > Keyboard shortcuts) ---------

fn sc_flag(sc: &Value, f: &str) -> bool {
    sc.get(f).and_then(Value::as_bool).unwrap_or(false)
}

/// `keybindingValueFromShortcut` equivalent: "mod+ctrl+meta+alt+shift+key".
fn sc_enc_shortcut(sc: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    if sc_flag(sc, "modKey") {
        parts.push("mod".into());
    }
    if sc_flag(sc, "ctrlKey") {
        parts.push("ctrl".into());
    }
    if sc_flag(sc, "metaKey") {
        parts.push("meta".into());
    }
    if sc_flag(sc, "altKey") {
        parts.push("alt".into());
    }
    if sc_flag(sc, "shiftKey") {
        parts.push("shift".into());
    }
    let key = sc.get("key").and_then(Value::as_str).unwrap_or("");
    parts.push(if key == " " { "space".into() } else { key.into() });
    parts.join("+")
}

fn sc_enc_captured(c: &ScCaptured) -> String {
    let mut parts: Vec<String> = Vec::new();
    if c.m_mod {
        parts.push("mod".into());
    }
    if c.m_ctrl {
        parts.push("ctrl".into());
    }
    if c.m_alt {
        parts.push("alt".into());
    }
    if c.m_shift {
        parts.push("shift".into());
    }
    parts.push(c.key.clone());
    parts.join("+")
}

/// Modifier-mask key used to group numbered-family bindings ("mod+alt").
fn sc_mod_mask(sc: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    if sc_flag(sc, "modKey") {
        parts.push("mod".into());
    }
    if sc_flag(sc, "ctrlKey") {
        parts.push("ctrl".into());
    }
    if sc_flag(sc, "metaKey") {
        parts.push("meta".into());
    }
    if sc_flag(sc, "altKey") {
        parts.push("alt".into());
    }
    if sc_flag(sc, "shiftKey") {
        parts.push("shift".into());
    }
    parts.join("+")
}

/// The config-form rule ({command, key, when}) edits operate on, rebuilt
/// from a resolved rule.
fn sc_rule_from_resolved(r: &Value) -> Value {
    let mut o = json!({
        "command": r.get("command").cloned().unwrap_or(Value::Null),
        "key": r
            .get("shortcut")
            .map(sc_enc_shortcut)
            .unwrap_or_default(),
    });
    let when_ast = r.get("whenAst").cloned().unwrap_or(Value::Null);
    if !when_ast.is_null() {
        if let Some(w) = sc_encode_when(&when_ast) {
            o["when"] = json!(w);
        }
    }
    o
}

/// `encodeKeybindingWhen` equivalent: AST -> expression text with the same
/// parenthesizing (left child wraps below precedence, right child at or
/// below it).
fn sc_encode_when(node: &Value) -> Option<String> {
    match node.get("type").and_then(Value::as_str)? {
        "identifier" => node.get("name").and_then(Value::as_str).map(|n| n.to_string()),
        "not" => {
            let child = node.get("node")?;
            let encoded = sc_encode_when(child)?;
            let needs_parens = matches!(
                child.get("type").and_then(Value::as_str),
                Some("and") | Some("or")
            );
            Some(if needs_parens {
                format!("!({encoded})")
            } else {
                format!("!{encoded}")
            })
        }
        kind @ ("and" | "or") => {
            let joiner = if kind == "or" { " || " } else { " && " };
            let parent_prec = if kind == "or" { 1 } else { 2 };
            let mut text = String::new();
            for side in ["left", "right"] {
                let child = node.get(side)?;
                let piece = sc_encode_when(child).unwrap_or_default();
                let child_prec = match child.get("type").and_then(Value::as_str) {
                    Some("or") => 1,
                    Some("and") => 2,
                    _ => 4,
                };
                let wrap = if side == "left" {
                    child_prec < parent_prec
                } else {
                    child_prec <= parent_prec
                };
                let piece = if wrap { format!("({piece})") } else { piece };
                if side == "right" {
                    text.push_str(joiner);
                }
                text.push_str(&piece);
            }
            Some(text)
        }
        _ => None,
    }
}

fn sc_key_label(key: &str) -> String {
    match key {
        " " | "space" => "Space".into(),
        "escape" | "esc" => "Esc".into(),
        "tab" => "Tab".into(),
        "arrowleft" | "left" => "\u{2190}".into(),
        "arrowright" | "right" => "\u{2192}".into(),
        "arrowup" | "up" => "\u{2191}".into(),
        "arrowdown" | "down" => "\u{2193}".into(),
        "comma" => ",".into(),
        "enter" | "return" => "\u{21a9}".into(),
        "backspace" => "\u{232b}".into(),
        "delete" | "forwarddelete" => "\u{2326}".into(),
        k if k.len() == 1 => k.to_uppercase(),
        k if k.starts_with('f') && k[1..].chars().all(|c| c.is_ascii_digit()) => {
            k.to_uppercase()
        }
        k => {
            let mut c = k.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => k.into(),
            }
        }
    }
}

/// macOS modifier order: ^ option shift cmd.
fn sc_mod_label(sc: &Value) -> String {
    let mut s = String::new();
    if sc_flag(sc, "ctrlKey") {
        s.push('\u{2303}');
    }
    if sc_flag(sc, "altKey") {
        s.push('\u{2325}');
    }
    if sc_flag(sc, "shiftKey") {
        s.push('\u{21e7}');
    }
    if sc_flag(sc, "modKey") || sc_flag(sc, "metaKey") {
        s.push('\u{2318}');
    }
    s
}

fn sc_fmt_label(sc: &Value) -> String {
    format!(
        "{}{}",
        sc_mod_label(sc),
        sc_key_label(sc.get("key").and_then(Value::as_str).unwrap_or(""))
    )
}

fn sc_fmt_captured(c: &ScCaptured) -> String {
    let mut s = String::new();
    if c.m_ctrl {
        s.push('\u{2303}');
    }
    if c.m_alt {
        s.push('\u{2325}');
    }
    if c.m_shift {
        s.push('\u{21e7}');
    }
    if c.m_mod {
        s.push('\u{2318}');
    }
    s.push_str(&sc_key_label(&c.key));
    s
}

fn sc_is_fkey(key: &str) -> bool {
    key.len() > 1
        && key.starts_with('f')
        && key[1..].chars().all(|c| c.is_ascii_digit())
}

/// Normalize rule-side key names to gpui's (`arrowleft` -> `left`,
/// " " -> `space`).
fn sc_norm_rule_key(k: &str) -> String {
    let k = k.to_lowercase();
    match k.as_str() {
        " " => "space".into(),
        "arrowleft" => "left".into(),
        "arrowright" => "right".into(),
        "arrowup" => "up".into(),
        "arrowdown" => "down".into(),
        _ => k,
    }
}

/// `when` expressions we can't evaluate: skip rules upstream reserves for
/// non-mac platforms; accept everything else (terminal focus is handled by
/// focus routing).
fn sc_when_ok(when: Option<&str>) -> bool {
    !when.map(|w| w.contains("!isMac")).unwrap_or(false)
}

fn sc_chip(label: impl Into<SharedString>, theme: &Theme) -> Div {
    div()
        .px(px(7.))
        .py(px(1.5))
        .rounded(px(10.))
        .bg(theme.secondary)
        .text_size(px(13.))
        .text_color(theme.muted_foreground)
        .child(label.into())
}

// Small ghost icon button matching the web sidebar-icon-button affordance.
/// Directories the explorer tree never expands (upstream
/// EXPLORER_HIDDEN_DIRECTORY_NAMES + `.synara*`). Files are always shown.
fn explorer_entry_visible(entry: &Value) -> bool {
    const HIDDEN_DIRS: &[&str] = &[
        ".cache", ".next", ".nuxt", ".parcel-cache", ".pnpm-store",
        ".svelte-kit", ".turbo", ".vite", ".yarn", "build", "coverage",
        "dist", "node_modules", "out", "target",
    ];
    if str_at(entry, "kind") != "directory" {
        return true;
    }
    let name = str_at(entry, "name");
    if name.starts_with(".synara") {
        return false;
    }
    !HIDDEN_DIRS.contains(&name)
}

// ── Inbox thread grouping (upstream inbox.logic.ts groupInboxThreads +
// Sidebar.logic.ts resolveThreadStatusPill). dismissedStatusKey and
// lastVisitedAt are not tracked natively yet: absent lastVisitedAt counts as
// unseen, matching upstream's fallback branch. ────────────────────────────

fn sess_str<'a>(t: &'a Value, key: &str) -> &'a str {
    str_at(get(t, "session"), key)
}

fn is_latest_turn_settled(lt: &Value, session: &Value) -> bool {
    if str_at(lt, "startedAt").is_empty() || str_at(lt, "completedAt").is_empty() {
        return false;
    }
    let st = str_at(lt, "state");
    if st == "interrupted" || st == "error" {
        return true;
    }
    if session.is_null() {
        return true;
    }
    str_at(session, "orchestrationStatus") != "running" && str_at(session, "status") != "running"
}

/// Upstream isThreadActivelyWorking.
fn thread_actively_working(t: &Value) -> bool {
    if get(t, "hasLiveTailWork").as_bool() == Some(true) {
        return true;
    }
    let session = get(t, "session");
    let lt = get(t, "latestTurn");
    str_at(session, "status") == "running"
        && (lt.is_null()
            || (!str_at(lt, "startedAt").is_empty() && !is_latest_turn_settled(lt, session)))
}

fn thread_running_for_activity(t: &Value) -> bool {
    thread_actively_working(t) || sess_str(t, "status") == "connecting"
}

/// Upstream isActivityThread: top-level, not archived, ran at least once
/// (or explicitly snoozed/reminded).
fn is_activity_thread(t: &Value) -> bool {
    if !get(t, "archivedAt").is_null() || !get(t, "parentThreadId").is_null() {
        return false;
    }
    !get(t, "snoozedUntil").is_null()
        || !get(t, "snoozeReminderAt").is_null()
        || !get(t, "latestTurn").is_null()
        || thread_running_for_activity(t)
}

/// Effective lastVisitedAt: max of the thread field and the locally
/// persisted map (upstream keeps both; markThreadVisited never regresses).
fn eff_visited(t: &Value, visited: &std::collections::HashMap<String, String>) -> String {
    let m = visited.get(str_at(t, "id")).map(|s| s.as_str()).unwrap_or("");
    let v = str_at(t, "lastVisitedAt");
    if m > v { m.to_string() } else { v.to_string() }
}

/// Upstream createThreadStatusDismissalKey:
/// [label, updatedAt, turnId, completedAt, session.updatedAt].join(":").
fn dismissal_key(label: &str, t: &Value) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        label,
        str_at(t, "updatedAt"),
        str_at(get(t, "latestTurn"), "turnId"),
        str_at(get(t, "latestTurn"), "completedAt"),
        str_at(get(t, "session"), "updatedAt")
    )
}

fn is_dismissed(label: &str, t: &Value, dismissed: &std::collections::HashMap<String, String>) -> bool {
    dismissed
        .get(str_at(t, "id"))
        .map(|k| *k == dismissal_key(label, t))
        .unwrap_or(false)
}

fn has_unseen_completion(t: &Value, visited: &std::collections::HashMap<String, String>) -> bool {
    let completed = str_at(get(t, "latestTurn"), "completedAt");
    if completed.is_empty() {
        return false;
    }
    let visited = eff_visited(t, visited);
    visited.is_empty() || completed > visited.as_str()
}

fn has_unseen_snooze_return(t: &Value, visited: &std::collections::HashMap<String, String>) -> bool {
    if !get(t, "snoozedUntil").is_null() {
        return false;
    }
    let reminder = str_at(t, "snoozeReminderAt");
    if reminder.is_empty() {
        return false;
    }
    let visited = eff_visited(t, visited);
    visited.is_empty() || reminder > visited.as_str()
}

fn can_answer_pending(t: &Value) -> bool {
    let session = get(t, "session");
    session.is_null()
        || (sess_str(t, "status") != "closed" && sess_str(t, "status") != "error")
}

/// Upstream needsYouKind: "approval" | "input" | "plan" | "failed" | "unread".
/// A dismissed pill is skipped and resolution continues to the next status.
fn inbox_needs_you_kind(
    t: &Value,
    dismissed: &std::collections::HashMap<String, String>,
    visited: &std::collections::HashMap<String, String>,
) -> Option<&'static str> {
    let can = can_answer_pending(t);
    let pending_approvals = get(t, "hasPendingApprovals").as_bool() == Some(true);
    let pending_input = get(t, "hasPendingUserInput").as_bool() == Some(true);
    if can && pending_approvals && !is_dismissed("Pending Approval", t, dismissed) {
        return Some("approval");
    }
    if can && pending_input && !is_dismissed("Awaiting Input", t, dismissed) {
        return Some("input");
    }
    if thread_actively_working(t) || sess_str(t, "status") == "connecting" {
        return None;
    }
    if get(t, "pendingBackgroundWorkCount").as_i64().unwrap_or(0) > 0 {
        return None;
    }
    let plan_ready = !(can && pending_input)
        && str_at(t, "interactionMode") == "plan"
        && is_latest_turn_settled(get(t, "latestTurn"), get(t, "session"))
        && get(t, "hasActionableProposedPlan").as_bool() == Some(true);
    if plan_ready && !is_dismissed("Plan Ready", t, dismissed) {
        return Some("plan");
    }
    if has_unseen_snooze_return(t, visited) {
        return Some("unread");
    }
    if has_unseen_completion(t, visited) {
        return Some(if str_at(get(t, "latestTurn"), "state") == "error" {
            "failed"
        } else {
            "unread"
        });
    }
    None
}

/// The status label upstream shows on the pill for a needs-you kind
/// (used for the dismissal key and the hover-card row).
fn needs_you_label(t: &Value, kind: &str) -> &'static str {
    match kind {
        "approval" => "Pending Approval",
        "input" => "Awaiting Input",
        "plan" => "Plan Ready",
        _ => {
            if !str_at(t, "snoozeReminderAt").is_empty()
                && str_at(get(t, "latestTurn"), "completedAt") <= str_at(t, "snoozeReminderAt")
            {
                "Reminder"
            } else {
                "Completed"
            }
        }
    }
}

/// Upstream groupInboxThreads: (needs_you kind+thread, working, finished, failed).
/// Sort order: kind rank (approval<input<plan<failed<unread) then updatedAt desc.
fn inbox_groups(
    threads: &[Value],
    dismissed: &std::collections::HashMap<String, String>,
    visited: &std::collections::HashMap<String, String>,
) -> (Vec<(&'static str, Value)>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut items: Vec<(u8, &'static str, Value)> = Vec::new();
    let mut listed = std::collections::HashSet::new();
    for t in threads {
        if !get(t, "snoozedUntil").is_null() || !is_activity_thread(t) {
            continue;
        }
        if let Some(kind) = inbox_needs_you_kind(t, dismissed, visited) {
            let rank = match kind {
                "approval" => 0,
                "input" => 1,
                "plan" => 2,
                "failed" => 3,
                _ => 4,
            };
            listed.insert(str_at(t, "id").to_string());
            items.push((rank, kind, t.clone()));
        }
    }
    let by_updated = |a: &Value, b: &Value| {
        str_at(b, "updatedAt")
            .cmp(str_at(a, "updatedAt"))
            .then(str_at(a, "id").cmp(str_at(b, "id")))
    };
    items.sort_by(|a, b| a.0.cmp(&b.0).then(by_updated(&a.2, &b.2)));
    let needs_you: Vec<(&'static str, Value)> = items
        .iter()
        .filter(|(_, k, _)| matches!(*k, "approval" | "input" | "plan"))
        .map(|(_, k, t)| (*k, t.clone()))
        .collect();
    let failed: Vec<Value> = items
        .iter()
        .filter(|(_, k, _)| *k == "failed")
        .map(|(_, _, t)| t.clone())
        .collect();
    let finished: Vec<Value> = items
        .iter()
        .filter(|(_, k, _)| *k == "unread")
        .map(|(_, _, t)| t.clone())
        .collect();
    let mut working: Vec<Value> = threads
        .iter()
        .filter(|t| {
            get(t, "snoozedUntil").is_null()
                && is_activity_thread(t)
                && !listed.contains(str_at(t, "id"))
                && thread_running_for_activity(t)
        })
        .cloned()
        .collect();
    working.sort_by(|a, b| by_updated(a, b));
    (needs_you, working, finished, failed)
}

fn icon_btn(id: &'static str, data: &'static [u8], size: f32, hover: Hsla, ink: Hsla) -> Stateful<Div> {
    icon_btn_pad(id, data, size, 16., hover, ink)
}

fn icon_btn_pad(
    id: &'static str,
    data: &'static [u8],
    size: f32,
    pad: f32,
    hover: Hsla,
    ink: Hsla,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(id.into()))
        .size(px(size + pad))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |d| d.bg(hover))
        .child(svg_icon(data).size(px(size)).text_color(ink))
}

type SharedClient = Arc<Mutex<Option<Arc<Client>>>>;

#[derive(Clone)]
struct ScCaptured {
    key: String,
    m_mod: bool,
    m_ctrl: bool,
    m_alt: bool,
    m_shift: bool,
}

/// Settings > Keyboard shortcuts recorder dialog state: the row being
/// bound, the resolved rules it replaces (empty = adding a binding), and
/// the chord captured so far.
struct ScRec {
    label: String,
    commands: Vec<String>,
    numbered: bool,
    replacing: Vec<Value>,
    captured: Option<ScCaptured>,
}



/// Drag payload for open-thread tab pills (reorder).
struct TabDrag(String);
/// Drag payload for sidebar thread rows (drop on the composer as a mention).
struct ThreadDrag(String);

/// Drag payload for explorer file rows (absolute path).
struct FileDrag(String);

/// The floating drag image: a small chip with the dragged item's title.
struct DragGhost(String);

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let ghost_bg: gpui::Hsla = if matches!(
            cx.window_appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ) {
            rgb(0x161616).into()
        } else {
            theme.popover
        };
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(ghost_bg)
            .border_1()
            .border_color(theme.border)
            .text_xs()
            .max_w(px(200.))
            .overflow_hidden()
            .whitespace_nowrap()
            .child(self.0.clone())
    }
}

/// Settings > Keyboard shortcuts rows: (title, description, default kbd
/// label, space-separated command ids) in upstream order.
static SHORTCUT_ROWS: [(&str, &str, &str, &str); 69] = [
    ("Toggle sidebar", "Collapse or reveal the sidebar shell.", "\u{2318}B", "sidebar.toggle"),
    ("Add project", "Open the Create project dialog to import a local folder.", "\u{21e7}\u{2318}O", "sidebar.addProject"),
    ("Search projects and threads", "Open the sidebar search palette from anywhere in the app.", "\u{2318}K", "sidebar.search"),
    ("Toggle Activity", "Show or hide running tasks, completed work, and items that need attention.", "\u{2325}\u{2318}U", "sidebar.activity"),
    ("Import thread", "Bring an existing conversation into the current workspace.", "\u{2318}I", "sidebar.importThread"),
    ("Previous space", "Switch to the previous project space and restore its last working context.", "\u{2325}\u{2318}Left", "space.previous"),
    ("Next space", "Switch to the next project space and restore its last working context.", "\u{2325}\u{2318}Right", "space.next"),
    ("New thread", "Start a fresh thread in the current project, or the most recent one.", "\u{2318}N", "chat.new"),
    ("New thread in latest project", "Jump back into the most recently used project with a new thread.", "\u{21e7}\u{2318}N", "chat.newLatestProject"),
    ("New chat", "Open the empty chat landing view.", "\u{2325}\u{2318}N", "chat.newChat chat.newLocal"),
    ("New terminal thread", "Create a thread that opens directly into terminal mode.", "\u{21e7}\u{2318}T", "chat.newTerminal"),
    ("New Claude thread", "Start a fresh thread with Claude selected.", "\u{2325}\u{2318}C", "chat.newClaude"),
    ("New Codex thread", "Start a fresh thread with Codex selected.", "\u{2325}\u{2318}X", "chat.newCodex"),
    ("New Cursor thread", "Start a fresh thread with Cursor selected.", "\u{2325}\u{2318}R", "chat.newCursor"),
    ("Split chat", "Open the current conversation in a second pane.", "\u{2318}\\", "chat.split"),
    ("Previous recent view", "Cycle backward through recently opened primary views.", "\u{2303}\u{21e7}Tab", "view.recent.previous"),
    ("Next recent view", "Cycle forward through recently opened primary views.", "\u{2303}Tab", "view.recent.next"),
    ("Model picker", "Open the composer provider and model picker.", "\u{21e7}\u{2318}M", "modelPicker.toggle"),
    ("Next model", "Cycle to the next model for the active provider (favorites first, then remaining models).", "\u{2325}]", "model.next"),
    ("Previous model", "Cycle to the previous model for the active provider (favorites first, then remaining models).", "\u{2325}[", "model.previous"),
    ("Next model effort", "Cycle the model's available effort levels and briefly show the selected level.", "\u{21e7}Tab", "model.effort.next"),
    ("Reasoning picker", "Open the composer reasoning and trait controls.", "\u{21e7}\u{2318}E", "traitsPicker.toggle"),
    ("Open usage settings", "Open Settings \u{2192} Usage for provider quota and token totals.", "\u{21e7}\u{2318}U", "settings.usage"),
    ("Focus composer", "Focus or blur the chat prompt composer.", "\u{2318}L", "composer.focus.toggle"),
    ("Find in thread", "Search the current transcript and jump to each matching message.", "\u{2318}F", "chat.find"),
    ("Search files", "Open the workspace file-name search palette.", "\u{2318}P", "search.files"),
    ("Search snippets", "Open the workspace content search palette across file snippets.", "\u{21e7}\u{2318}F", "search.content"),
    ("Toggle terminal", "Show or hide the terminal surface for the active thread.", "\u{2318}J", "terminal.toggle"),
    ("Focus terminal", "Open or focus the terminal panel for this thread.", "\u{2318}T", "terminal.new"),
    ("Close terminal", "Close the terminal session for this thread.", "\u{2318}W", "terminal.close"),
    ("Toggle diff", "Open or close the working tree diff panel.", "\u{2318}D", "diff.toggle"),
    ("Next change", "Jump the diff viewport to the next changed file.", "\u{2325}Down", "diff.change.next"),
    ("Previous change", "Jump the diff viewport to the previous changed file.", "\u{2325}Up", "diff.change.previous"),
    ("Toggle side chat", "Open or hide a side chat beside the main conversation.", "\u{2325}\u{2318}S", "sidechat.toggle"),
    ("Toggle browser", "Reveal the built-in browser panel for the active thread.", "\u{21e7}\u{2318}B", "browser.toggle"),
    ("Toggle iOS Simulator", "Reveal the iOS Simulator panel for the active thread. macOS servers only.", "", "device.toggle"),
    ("Copy thread ID", "Copy the active thread's ID to the clipboard.", "\u{21e7}\u{2318}C", "thread.copyId"),
    ("Archive thread", "Archive the active chat, with the same confirmation and undo as the thread menu.", "\u{2325}\u{21e7}\u{2318}A", "thread.archive"),
    ("Snooze thread", "Open the snooze date and time picker for the active chat.", "\u{2325}\u{21e7}\u{2318}S", "thread.snooze"),
    ("Mark thread unread", "Mark the active chat unread and restore its dismissed notification.", "\u{2325}\u{21e7}\u{2318}U", "thread.markUnread"),
    ("Previous visible thread", "Cycle to the previous thread that is currently visible in the sidebar.", "\u{21e7}\u{2318}[", "chat.visible.previous"),
    ("Next visible thread", "Cycle to the next thread that is currently visible in the sidebar.", "\u{21e7}\u{2318}]", "chat.visible.next"),
    ("Next chat needing attention", "Open the most urgent other chat: one waiting on you first, then unseen work, as Activity's Priority ranks them.", "\u{2325}\u{2318}A", "chat.attention.next"),
    ("Previous tab", "Switch to the open thread tab on the left of the active one.", "\u{2303}\u{2318}Left", "threadTab.previous"),
    ("Next tab", "Switch to the open thread tab on the right of the active one.", "\u{2303}\u{2318}Right", "threadTab.next"),
    ("Close tab", "Close the active open-thread tab, like a browser tab.", "\u{2318}W", "threadTab.close"),
    ("Open in favorite editor", "Send the current thread or workspace target to your preferred editor.", "\u{2318}O", "editor.openFavorite"),
    ("Save file", "Write the focused editor's unsaved changes back to disk.", "\u{2318}S", "editor.file.save"),
    ("Commit and push", "Commit pending changes and push the active thread's repo.", "\u{2303}\u{2318}P", "git.commitAndPush"),
    ("Editor commands", "Open the editor command picker for the focused code editor.", "F1", "editor.commands"),
    ("Rename symbol", "Rename the symbol under the cursor across the workspace.", "F2", "editor.rename"),
    ("Go to definition", "Jump to the definition of the symbol under the cursor.", "F12", "editor.goToDefinition"),
    ("Find references", "List references to the symbol under the cursor.", "\u{21e7}F12", "editor.findReferences"),
    ("Go to symbol in file", "Jump to a symbol in the open file.", "", "editor.goToSymbol"),
    ("Go to line", "Jump to a line, or line:column, in the open file.", "", "editor.goToLine"),
    ("Show problems", "List diagnostics for the open file.", "", "editor.problems"),
    ("Quick fix", "Show quick fixes and refactorings for the code at the cursor.", "\u{2318}.", "editor.quickFix"),
    ("Start or continue debugging", "Launch the debugger for the open file, or continue a paused session.", "F5", "editor.debug.start"),
    ("Stop debugging", "Stop the current debug session.", "\u{21e7}F5", "editor.debug.stop"),
    ("Toggle breakpoint", "Add or remove a breakpoint on the cursor line.", "F9", "editor.debug.toggleBreakpoint"),
    ("Step over", "Run the next statement without entering calls.", "F10", "editor.debug.stepOver"),
    ("Step into", "Enter the next call.", "F11", "editor.debug.stepInto"),
    ("Step out", "Run until the current call returns.", "\u{21e7}F11", "editor.debug.stepOut"),
    ("Open full-width terminal workspace", "Expand the active thread into the workspace terminal layout.", "\u{21e7}\u{2318}J", "terminal.workspace.newFullWidth"),
    ("Focus terminal tab", "Switch the workspace to the terminal tab.", "\u{2303}1", "terminal.workspace.terminal"),
    ("Focus chat tab", "Switch the workspace back to the chat tab.", "\u{2303}2", "terminal.workspace.chat"),
    ("Close active workspace panel", "Close the currently focused workspace panel or tab.", "\u{2318}W", "terminal.workspace.closeActive"),
    ("Jump to visible thread 1\u{2013}9", "Focus a visible thread directly from the sidebar number row.", "\u{2318}1\u{2013}9", "thread.jump.1 thread.jump.2 thread.jump.3 thread.jump.4 thread.jump.5 thread.jump.6 thread.jump.7 thread.jump.8 thread.jump.9"),
    ("Jump to space 1\u{2013}9", "Switch straight to a tab of the space switcher. 1 is always Void.", "\u{2325}\u{2318}1\u{2013}9", "space.jump.1 space.jump.2 space.jump.3 space.jump.4 space.jump.5 space.jump.6 space.jump.7 space.jump.8 space.jump.9"),
];

pub struct Native {
    focus_handle: FocusHandle,
    client: SharedClient,
    snapshot: Option<Value>,
    detail: Option<Value>,
    selected_thread: Option<String>,
    thread_stream_id: Option<String>,
    composer: Entity<InputState>,
    search: Entity<InputState>,
    settings_search: Entity<InputState>,
    /// Files-pane fuzzy search box (upstream WorkspaceSearchInputHeader).
    file_search: Entity<InputState>,
    /// Latest `projects.searchEntries` result rows (path is relative to files_dir).
    file_search_results: Vec<Value>,
    /// Monotonic guard dropping stale debounced search replies.
    file_search_seq: u64,
    sending: bool,
    view_files: bool,
    files_dir: String,
    files: Vec<Value>,
    /// Explorer tree children per directory path (upstream WorkspaceDirectory:
    /// each expanded directory lazily fetches its own listing, cached here so
    /// collapse/expand never refetches).
    tree_entries: std::collections::HashMap<String, Vec<Value>>,
    /// Expanded directory paths in the explorer tree.
    expanded_dirs: std::collections::HashSet<String>,
    /// Directory listings currently in flight for the tree.
    tree_loading: std::collections::HashSet<String>,
    /// Composer "@" mention browser (upstream local-folder mentions):
    /// entries of the directory the current @query points into.
    mention_dir: String,
    mention_entries: Vec<Value>,
    /// Drop stale `projects.listDirectories` replies: fast typing issues
    /// one browse per keystroke and replies can land out of order.
    mention_seq: u64,
    open_file: Option<(String, String)>,
    open_file_meta: Option<(String, String, String)>,
    file_editor: Option<Entity<EditorState>>,
    editor_synced: Option<String>,
    editing_file: bool,
    file_dirty: bool,
    open_in_menu: bool,
    files_loading: bool,
    git_status: Option<Value>,
    file_stream_id: Option<String>,
    search_query: String,
    search_hits: Vec<Value>,
    view_dev: bool,
    dev_servers: Vec<Value>,
    dev_cmd: Entity<InputState>,
    runtime_mode: &'static str,
    chats_collapsed: bool,
    /// Project folders the user collapsed in the sidebar (upstream
    /// toggleProject). Projects start expanded.
    collapsed_projects: std::collections::HashSet<String>,
    view_settings: bool,
    settings_section: &'static str,
    view_inbox: bool,
    view_tasks: bool,
    view_kanban: bool,
    view_pullreq: bool,
    view_autom: bool,
    view_studio: bool,
    view_spaces: bool,
    inbox_items: Vec<Value>,
    inbox_errors: Vec<Value>,
    inbox_loading: bool,
    inbox_tasks: Vec<Value>,
    inbox_recap: Option<Value>,
    inbox_usage: Vec<Value>,
    inbox_signed_in: Option<bool>,
    inbox_updates: Vec<Value>,
    tools_expanded: bool,
    pending_attachments: Vec<Value>,
    composer_thread_id: Option<String>,
    uploading: bool,
    interaction_mode: &'static str,
    fast_mode: bool,
    queued_turns: Vec<String>,
    queue_paused: bool,
    open_threads: Vec<String>,
    pinned_threads: Vec<String>,
    /// Insertion index for an in-flight tab-pill drag (0..=len), or None.
    tab_drop_target: Option<usize>,
    split_open: bool,
    split_thread: Option<String>,
    split_detail: Option<Value>,
    split_stream_id: Option<String>,
    focused_pane: u8,
    split_ratio: f32,
    split_dragging: bool,
    split_composer: Option<Entity<InputState>>,
    /// True while a thread drag hovers the composer (ring highlight).
    composer_thread_over: bool,
    transcript_scroll: ScrollHandle,
    editing_message_id: Option<String>,
    pending_revert: Option<(String, i64)>,
    /// Appearance settings (upstream appSettings, persisted under
    /// settings.appearance via server.updateSettings).
    appearance_mode: u8,
    ui_density: u8,
    chat_width_mode: u8,
    show_ascii_bg: bool,
    /// AsciiWaveBackground (upstream): ambient field clock. `ascii_cells`
    /// holds the per-cell glyph indices so scrambling is incremental, and
    /// `ascii_ripples` the live click ripples (x, y, phaseA, phaseB, start).
    ascii_t0: std::time::Instant,
    ascii_seed: f32,
    ascii_cells: Vec<u8>,
    ascii_ripples: Vec<(f32, f32, f32, f32, std::time::Instant)>,
    /// Set when the landing leaves the screen; the next landing mounts a
    /// fresh field (upstream remounts AsciiWaveBackground).
    ascii_fresh: bool,
    renaming: bool,
    rename_input: Option<Entity<InputState>>,
    view_changes: bool,
    staged_files: Vec<Value>,
    unstaged_files: Vec<Value>,
    changes_diff: Option<(String, String)>,
    /// Diff-line blame popover (upstream DiffLineBlamePopover):
    /// (`git.blameLine` result, x, y).
    blame_pop: Option<(Value, f32, f32)>,
    changes_loading: bool,
    changes_unavailable: bool,
    // Right dock pane tabs (upstream RightDock/SurfaceContentTabs):
    // 0 = Git, 1 = Subagents, 2 = Background tasks, 3 = Side chats.
    dock_tab: usize,
    view_terminal: bool,
    term_active: bool,
    // TerminalWorkspaceTabs: one PTY session per tab; events route by id.
    term_tabs: Vec<TermTab>,
    term_sel: usize,
    settings_scroll: ScrollHandle,
    settings_nav_scroll: ScrollHandle,
    dock_tabs_scroll: ScrollHandle,
    settings_data: Value,
    settings_loading: bool,
    /// Upstream dismissedThreadStatusKeyByThreadId (thread_id -> dismissalKey).
    dismissed_status: std::collections::HashMap<String, String>,
    /// Upstream lastVisitedAtByThreadId (thread_id -> ISO timestamp).
    visited_threads: std::collections::HashMap<String, String>,
    /// Upstream keepAwakeMode setting: 0 on, 1 agent, 2 off.
    keep_awake_mode: usize,
    /// Upstream OneTimeCoachmark: seen keys persisted under
    /// settings.sidebarUi.coachmarks; `coachmark` is the one showing.
    coachmarks_seen: std::collections::HashSet<String>,
    // Upstream DebugFeatureFlagsMenu: [persist-toasts, pin-git-progress,
    // show-debug-task-banner]; sidebar footer popup, dev-only surface.
    feature_flags: [bool; 3],
    flags_menu_open: bool,
    coachmark: Option<&'static str>,
    term_input: Option<Entity<InputState>>,
    term_events: bool,
    // Working directory "Open Path in Terminal" seeds the next terminal.open
    // with when the drawer is currently closed.
    term_pending_cwd: Option<String>,
    // TerminalSearch: in-buffer find bar (magnifier toggles it).
    term_search_input: Option<Entity<InputState>>,
    term_search_query: String,
    find_open: bool,
    find_input: Option<Entity<InputState>>,
    find_query: String,
    sponsor_dismissed: bool,
    search_open: bool,
    sidebar_collapsed: bool,
    // Async user-input (pending question) cards: expanded ids and the
    // (message_id, question_index) -> chosen option label draft.
    user_input_open: Vec<String>,
    ctx_menu: Option<(String, f32, f32)>,
    // Context-menu flyout submenu: (parent row id, flyout top y) while open,
    // and the thread id awaiting inline delete confirmation.
    ctx_sub: Option<(String, f32)>,
    /// File-row context menu (upstream showFileReferenceContextMenu):
    /// (absolute path, x, y).
    file_ctx_menu: Option<(String, f32, f32)>,
    /// Sidebar thread row the pointer is over — reveals the hover
    /// quick-actions (upstream ThreadRowQuickActions).
    hover_thread: Option<String>,
    /// Hover-card popup: (thread id, x, y) after a short hover delay
    /// (upstream ThreadHoverCardContent tooltip).
    hover_card: Option<(String, f32, f32)>,
    hover_card_seq: u64,
    ctx_confirm_delete: Option<String>,
    /// Project context menu (upstream handleProjectContextMenu):
    /// (project id, x, y); `proj_ctx_confirm` arms the inline remove confirm.
    proj_ctx_menu: Option<(String, f32, f32)>,
    proj_ctx_confirm: bool,
    /// Project hover card (upstream ProjectHoverCardContent): hovered id and
    /// the delayed card (id, x, y).
    hover_project: Option<String>,
    project_card: Option<(String, f32, f32)>,
    project_card_seq: u64,
    /// Project rename dialog (upstream EditProjectDialog): id + input entity.
    proj_rename: Option<String>,
    proj_rename_input: Option<Entity<InputState>>,
    // Workspace search palette (⌘K chats / ⌘P files / ⇧⌘F snippets): open
    // mode, input entity, last async result rows, highlighted row, and a
    // sequence counter that drops stale search responses.
    palette_mode: Option<&'static str>,
    palette_input: Option<Entity<InputState>>,
    palette_rows: Vec<Value>,
    palette_sel: usize,
    palette_seq: u64,
    // Resolved keybinding rules from `server.getConfig`
    // ({command, shortcut:{key,*Key}, when}) — drives both live Settings
    // rows and global shortcut dispatch once loaded.
    kb_rules: Vec<Value>,
    // Shortcut recorder dialog state (Settings > Keyboard shortcuts).
    sc_rec: Option<ScRec>,
    // Feedback dialog (upstream FeedbackDialog): category pill index into
    // FEEDBACK_CATEGORIES, details textarea, in-flight submit, result toast.
    feedback_open: bool,
    feedback_category: Option<usize>,
    feedback_input: Option<Entity<InputState>>,
    feedback_sending: bool,
    feedback_toast: Option<(String, bool)>,
    // Global keybindings reference dialog (upstream ShortcutsDialog, mod+/).
    sc_dialog_open: bool,
    sc_dialog_input: Option<Entity<InputState>>,
    /// App tour walkthrough (upstream AppTour): current step index.
    tour_step: Option<usize>,
    user_input_draft: Vec<(String, usize, String)>,
    // Git actions (upstream GitActionsControl + GitCommitDialog/GitCreatePrDialog).
    git_dialog: Option<GitDialog>,
    git_msg_input: Option<Entity<InputState>>,
    git_pr_title_input: Option<Entity<InputState>>,
    git_pr_body_input: Option<Entity<InputState>>,
    git_pr_draft: bool,
    git_commit_action: String,
    git_action_running: bool,
    // Automations rail pane (upstream RailAutomationsPanel + automation form).
    autom_defs: Vec<Value>,
    autom_runs: Vec<Value>,
    autom_sel: Option<String>,
    autom_dialog: Option<AutomForm>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GitDialog {
    Commit,
    Pr,
}

/// One PTY session backing a terminal tab (upstream TerminalWorkspaceTabs).
struct TermTab {
    id: String,
    buf: String,
    status: Option<String>,
}

impl Default for TermTab {
    fn default() -> Self {
        Self {
            id: uuid(),
            buf: String::new(),
            status: None,
        }
    }
}

struct AutomForm {
    edit_id: Option<String>,
    name: Entity<InputState>,
    prompt: Entity<InputState>,
    time: Entity<InputState>,
    extra: Entity<InputState>,
    kind: usize,
    day: usize,
    proj: usize,
    enabled: bool,
}

const AUTOM_KINDS: &[&str] = &["Once", "Interval", "Daily", "Weekdays", "Weekly", "Cron"];
const AUTOM_DAYS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const SETTINGS_SEARCH_ORDER: &[(&str, &str)] = &[
    ("profile", "Profile"),
    ("accounts", "Accounts"),
    ("general", "General"),
    ("appearance", "Appearance"),
    ("layout", "Layout"),
    ("chats", "Chats"),
    ("notifications", "Notifications"),
    ("shortcuts", "Keyboard shortcuts"),
    ("providers", "Providers"),
    ("models", "Models"),
    ("skills", "Skills"),
    ("usage", "Usage"),
    ("connectors", "Connectors"),
    ("integrations", "External agents"),
    ("devices", "Devices"),
    ("computer", "Computer use"),
    ("appsnap", "AppSnap"),
    ("worktrees", "Worktrees"),
    ("archive", "Archive"),
    ("privacy", "Privacy"),
    ("advanced", "Advanced"),
    ("about", "About"),
];

// App tour steps (upstream APP_TOUR_STEPS, copy verbatim from locale JSON):
// (place label, title, description, anchor rect in window coords or None for
// a centered card). Anchors match the native rail/sidebar layout.
const TOUR_STEPS: [(&str, &str, &str, Option<(f32, f32, f32, f32)>); 5] = [
    (
        "New thread",
        "Start a chat with any agent",
        "Pick the agent and model for each chat, and hand the work to another without leaving the thread.",
        Some((57.0, 96.0, 300.0, 128.0)),
    ),
    (
        "Activity",
        "See what your agents are doing",
        "See running tasks, completed work, and anything that needs your attention.",
        None,
    ),
    (
        "Tasks",
        "Keep your to-dos in Tasks",
        "Keep your to-dos in one place and hand them to an agent when you\u{2019}re ready.",
        Some((8.0, 175.0, 44.0, 211.0)),
    ),
    (
        "Inbox",
        "Start the day in Inbox",
        "Today\u{2019}s tasks, review requests, and a recap of what your agents did.",
        Some((8.0, 91.0, 44.0, 127.0)),
    ),
    (
        "Code review",
        "Review pull requests and issues",
        "Browse pull requests and issues, discuss them in a side chat, and ask an agent to review.",
        Some((8.0, 217.0, 44.0, 253.0)),
    ),
];

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn provider_icon(provider: &str) -> &'static [u8] {
    match provider {
        "pi" => icons::P_PI,
        "claude" => icons::P_CLAUDE,
        "devin" => icons::P_DEVIN,
        "cursor" => icons::P_CURSOR,
        "opencode" => icons::P_OPENCODE,
        "omp" => icons::P_OMP,
        "droid" => icons::P_DROID,
        "antigravity" => icons::P_ANTIGRAVITY,
        "grok" => icons::P_GROK,
        _ => icons::CODEX,
    }
}

fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    get(v, k).as_str().unwrap_or("")
}

/// Upstream derivePhase(session): closed->disconnected, connecting->connecting,
/// running->running, otherwise ready.
fn session_phase(detail: Option<&Value>) -> &'static str {
    let status = detail
        .map(|d| get(d, "thread"))
        .map(|t| get(t, "session"))
        .and_then(|s| s.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("");
    match status {
        "running" => "running",
        "connecting" => "connecting",
        "closed" => "disconnected",
        _ => "ready",
    }
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k).as_array().map(Vec::as_slice).unwrap_or(&[])
}

// ── AsciiWaveBackground (upstream ambient field) ────────────────────────────
// Port of apps/web/src/components/chat/AsciiWaveBackground.tsx: three cycling
// noise patterns, per-cell glyph scramble, and accent click ripples.

const ASCII_GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const ASCII_COLS: usize = 160;
const ASCII_ROWS: usize = 72;
// Cell pitch in px, matching upstream FONT_SIZE_PX/LINE_HEIGHT_PX metrics.
const ASCII_CELL_W: f32 = 6.0;
const ASCII_CELL_H: f32 = 13.0;
const ASCII_PATTERN_MS: u64 = 20_000;
const ASCII_CROSSFADE_MS: u64 = 4_000;
const ASCII_REVEAL_MS: u64 = 1_200;
const ASCII_RIPPLE_LIFE_MS: u64 = 2_600;
const ASCII_RIPPLE_MAX: usize = 6;

// (scaleX, scaleY, driftX, driftY, waveX, waveY, waveLength, waveSpeed, radial)
type AsciiPattern = (f32, f32, f32, f32, f32, f32, f32, f32, bool);
const ASCII_PATTERNS: [AsciiPattern; 3] = [
    (190., 150., 0.1, -0.06, 0.8, 0.6, 70., 1.3, false), // Blotches
    (360., 80., 0.16, 0.01, 0.25, 1., 38., 1.8, false),  // Currents
    (150., 150., -0.06, 0.09, 0., 0., 55., 1.5, true),   // Rings
];

fn ascii_hash3(x: i64, y: i64, z: i64) -> f32 {
    // hash3(): Math.imul mixes, exact u32 arithmetic.
    let h = (x as i32)
        .wrapping_mul(374761393)
        .wrapping_add((y as i32).wrapping_mul(668265263))
        .wrapping_add((z as i32).wrapping_mul(1274126177));
    let h = (h ^ ((h as u32 >> 13) as i32)).wrapping_mul(1274126177);
    ((h as u32) ^ ((h as u32) >> 16)) as f32 / 4294967296.0
}

fn smoothstep(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn lerp_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn noise_plane(x0: i64, y0: i64, z: i64, fx: f32, fy: f32) -> f32 {
    lerp_f(
        lerp_f(
            ascii_hash3(x0, y0, z),
            ascii_hash3(x0 + 1, y0, z),
            fx,
        ),
        lerp_f(
            ascii_hash3(x0, y0 + 1, z),
            ascii_hash3(x0 + 1, y0 + 1, z),
            fx,
        ),
        fy,
    )
}

fn value_noise3(x: f32, y: f32, z: f32) -> f32 {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let z0 = z.floor() as i64;
    let fx = smoothstep(0., 1., x - x0 as f32);
    let fy = smoothstep(0., 1., y - y0 as f32);
    lerp_f(
        noise_plane(x0, y0, z0, fx, fy),
        noise_plane(x0, y0, z0 + 1, fx, fy),
        smoothstep(0., 1., z - z0 as f32),
    )
}

fn ascii_sample(p: &AsciiPattern, x: f32, y: f32, cx: f32, cy: f32, t: f32) -> (f32, f32) {
    let (sx, sy, dx, dy, wx, wy, wl, ws, radial) = *p;
    let noise = 0.68
        * value_noise3(x / sx + t * dx, y / sy + t * dy, t * 0.18)
        + 0.32 * value_noise3(x / 70. - t * 0.08, y / 52., t * 0.35 + 17.);
    let band = if radial {
        ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt()
    } else {
        x * wx + y * wy
    };
    let wave = 0.5 + 0.5 * (band / wl - t * ws + noise * 5.).sin();
    let patch = smoothstep(0.4, 0.78, noise + 0.1 * (wave - 0.5));
    (patch, wave)
}

/// One derived background task for the right-dock Tasks pane — the port of
/// upstream `deriveThreadBackgroundTasks` (workLog.ts): a task becomes a
/// background task on a "Moved to background" notice or a backgrounded update;
/// task.started links the launching call, task.updated/completed settle it.
struct BgTask {
    task_id: String,
    title: String,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    exit_code: Option<i64>,
}

fn bg_task_outcome(status: &str) -> Option<&'static str> {
    match status {
        "finished" => Some("finished"),
        "failed" => Some("failed"),
        "stopped" => Some("stopped"),
        _ => None,
    }
}

fn derive_bg_tasks(thread: &Value) -> Vec<BgTask> {
    use std::collections::HashMap;
    struct Launch {
        tool_use_id: Option<String>,
        task_type: Option<String>,
        description: Option<String>,
        started_at: String,
    }
    struct State {
        description: Option<String>,
        command: Option<String>,
        status: String,
        started_at: String,
        completed_at: Option<String>,
        exit_code: Option<i64>,
        tool_use_id: Option<String>,
    }
    let mut tasks: HashMap<String, State> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut started: HashMap<String, Launch> = HashMap::new();
    let admit = |tasks: &mut HashMap<String, State>,
                     order: &mut Vec<String>,
                     started: &HashMap<String, Launch>,
                     task_id: &str,
                     task_type: Option<String>,
                     description: Option<String>,
                     created_at: &str| {
        let launch = started.get(task_id);
        let task_type = task_type.or_else(|| launch.and_then(|l| l.task_type.clone()));
        // Subagents keep their own transcript row; monitors are excluded upstream.
        if task_type.as_deref() == Some("monitor") || task_type.as_deref() == Some("local_agent") {
            tasks.remove(task_id);
            order.retain(|id| id != task_id);
            return;
        }
        if tasks.contains_key(task_id) {
            return;
        }
        order.push(task_id.to_string());
        tasks.insert(
            task_id.to_string(),
            State {
                description: description.or_else(|| launch.and_then(|l| l.description.clone())),
                command: None,
                status: "running".to_string(),
                started_at: launch
                    .map(|l| l.started_at.clone())
                    .unwrap_or_else(|| created_at.to_string()),
                completed_at: None,
                exit_code: None,
                tool_use_id: launch.and_then(|l| l.tool_use_id.clone()),
            },
        );
    };

    let activities = arr(thread, "activities");
    for a in activities {
        let kind = str_at(a, "kind");
        let payload = get(a, "payload");
        let created_at = str_at(a, "createdAt").to_string();
        if kind == "runtime.warning"
            && str_at(payload, "nativeEventType") == "background_tasks_changed"
        {
            for t in arr(get(payload, "data"), "tasks") {
                let tid = str_at(t, "task_id").to_string();
                if tid.is_empty() {
                    continue;
                }
                let ty = {
                    let s = str_at(t, "task_type");
                    if s.is_empty() { None } else { Some(s.to_string()) }
                };
                let desc = {
                    let s = str_at(t, "description");
                    if s.is_empty() { None } else { Some(s.to_string()) }
                };
                admit(&mut tasks, &mut order, &started, &tid, ty, desc, &created_at);
            }
            continue;
        }
        let tid = str_at(payload, "taskId").to_string();
        if tid.is_empty() {
            continue;
        }
        if kind == "task.started" {
            started.insert(
                tid.clone(),
                Launch {
                    tool_use_id: {
                        let s = str_at(payload, "toolUseId");
                        if s.is_empty() { None } else { Some(s.to_string()) }
                    },
                    task_type: {
                        let s = str_at(payload, "taskType");
                        if s.is_empty() { None } else { Some(s.to_string()) }
                    },
                    description: {
                        let s = str_at(payload, "detail");
                        if s.is_empty() { None } else { Some(s.to_string()) }
                    },
                    started_at: created_at.clone(),
                },
            );
            let is_agent = started
                .get(&tid)
                .and_then(|l| l.task_type.as_deref())
                == Some("local_agent");
            if is_agent {
                admit(&mut tasks, &mut order, &started, &tid, Some("local_agent".into()), None, &created_at);
            } else if let Some(task) = tasks.get_mut(&tid) {
                if task.tool_use_id.is_none() {
                    task.tool_use_id = started.get(&tid).and_then(|l| l.tool_use_id.clone());
                }
                if task.description.is_none() {
                    let d = str_at(payload, "detail");
                    if !d.is_empty() {
                        task.description = Some(d.to_string());
                    }
                }
            }
            continue;
        }
        if kind == "task.updated" && get(payload, "isBackgrounded").as_bool() == Some(true) {
            let ty = {
                let s = str_at(payload, "taskType");
                if s.is_empty() { None } else { Some(s.to_string()) }
            };
            let desc = {
                let s = str_at(payload, "detail");
                if s.is_empty() { None } else { Some(s.to_string()) }
            };
            admit(&mut tasks, &mut order, &started, &tid, ty, desc, &created_at);
        }
        let outcome = bg_task_outcome(str_at(payload, "status"));
        let reconciles_stop = get(payload, "reconcilesStop").as_bool() == Some(true)
            && tasks.get(&tid).map(|t| t.status.as_str()) == Some("stopped");
        let Some(task) = tasks.get_mut(&tid) else { continue };
        if !(task.status == "running" || reconciles_stop) {
            continue;
        }
        if reconciles_stop && kind == "task.updated" && str_at(payload, "status") == "running" {
            task.status = "running".to_string();
            task.completed_at = None;
            task.exit_code = None;
            continue;
        }
        if kind == "task.updated" || kind == "task.completed" {
            let Some(oc) = outcome else { continue };
            task.status = oc.to_string();
            task.completed_at = Some(created_at.clone());
            let detail = str_at(payload, "detail");
            task.exit_code = detail
                .to_lowercase()
                .find("exit code ")
                .and_then(|i| detail[i + 10..].split_whitespace().next()?.parse::<i64>().ok());
        }
    }
    // The launching call's command wins when it names one (upstream merges the
    // launch notice with the tool-call command line).
    let launch_ids: std::collections::HashSet<String> = tasks
        .values()
        .filter_map(|t| t.tool_use_id.clone())
        .collect();
    if !launch_ids.is_empty() {
        for a in activities {
            let payload = get(a, "payload");
            let call_id = {
                let s = str_at(payload, "toolCallId");
                if s.is_empty() { str_at(payload, "toolUseId") } else { s }
            };
            if call_id.is_empty() || !launch_ids.contains(call_id) {
                continue;
            }
            let data = get(payload, "data");
            let item = get(data, "item");
            let candidates = [
                str_at(get(item, "input"), "command"),
                str_at(get(data, "input"), "command"),
                str_at(get(get(data, "rawInput"), ""), "command"),
                str_at(payload, "command"),
            ];
            if let Some(cmd) = candidates.iter().find(|c| !c.is_empty()) {
                for task in tasks.values_mut() {
                    if task.tool_use_id.as_deref() == Some(call_id) {
                        task.command = Some(cmd.to_string());
                    }
                }
            }
        }
    }
    order
        .iter()
        .filter_map(|id| tasks.get(id).map(|t| (id.clone(), t)))
        .map(|(id, t)| BgTask {
            task_id: id,
            title: t
                .command
                .clone()
                .or_else(|| t.description.clone())
                .unwrap_or_else(|| "Background task".to_string()),
            status: t.status.clone(),
            started_at: t.started_at.clone(),
            completed_at: t.completed_at.clone(),
            exit_code: t.exit_code,
        })
        .collect()
}

/// The "@" token the draft ends in, if the composer is in a mention trigger:
/// "@" must start a word (preceded by whitespace or at index 0) and no
/// whitespace may follow it — like upstream's composerTrigger "mention".
fn mention_query(draft: &str) -> Option<String> {
    let at = draft.rfind('@')?;
    if at > 0 && !draft[..at].ends_with(char::is_whitespace) {
        return None;
    }
    let q = &draft[at + 1..];
    if q.contains(char::is_whitespace) {
        return None;
    }
    Some(q.to_string())
}

/// Directory the mention browser should list for the current draft, or None
/// when no mention trigger is active. Path-like queries (contain '/',
/// upstream isLocalFolderMentionQuery) browse their dirname; plain queries
/// browse the home directory.
fn mention_browse_dir(draft: &str) -> Option<String> {
    let q = mention_query(draft)?;
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    if !q.contains('/') {
        return Some(home);
    }
    let expanded = q
        .strip_prefix("~/")
        .map(|rest| format!("{home}/{rest}"))
        .unwrap_or_else(|| q.clone());
    let dir = expanded
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "/".to_string());
    Some(dir)
}

fn get_mut<'a>(v: &'a mut Value, k: &str) -> Option<&'a mut Value> {
    v.get_mut(k)
}

/// Split a unified diff patch into per-file rows (path, +ins, -del), like
/// upstream's parsePatchToSortedFiles on `git readWorkingTreeDiff` output.
fn parse_patch_files(patch: &str) -> Vec<Value> {
    let mut files = Vec::new();
    for part in patch.split("diff --git ") {
        let header = part.lines().next().unwrap_or("");
        if !header.starts_with("a/") {
            continue;
        }
        let path = header
            .split(" b/")
            .nth(1)
            .map(|s| s.trim().to_string())
            .or_else(|| {
                header
                    .trim_start_matches("a/")
                    .split(' ')
                    .next()
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        let mut ins = 0i64;
        let mut del = 0i64;
        for line in part.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                ins += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                del += 1;
            }
        }
        files.push(json!({
            "path": path,
            "insertions": ins,
            "deletions": del,
        }));
    }
    files
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn rel_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let d = chrono::Utc::now().signed_duration_since(t).num_seconds().max(0);
    if d < 3600 {
        format!("{}m", d / 60)
    } else if d < 86400 {
        format!("{}h", d / 3600)
    } else {
        format!("{}d", d / 86400)
    }
}

// Local wall-clock time "h:mm AM/PM" like the upstream message footer.
fn clock_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    t.with_timezone(&chrono::Local).format("%-I:%M %p").to_string()
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Drop ANSI escape sequences (CSI/OSC/single-char) and CR/BS/BEL from PTY
/// output — the drawer renders plain text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            match it.peek().copied() {
                Some('[') => {
                    it.next();
                    while let Some(n) = it.next() {
                        if n.is_ascii_alphabetic() || n == '@' || n == '`' {
                            break;
                        }
                    }
                }
                Some(']') => {
                    it.next();
                    loop {
                        match it.next() {
                            None | Some('\u{7}') => break,
                            Some('\u{1b}') => {
                                if it.peek() == Some(&'\\') {
                                    it.next();
                                }
                                break;
                            }
                            Some(_) => {}
                        }
                    }
                }
                _ => {
                    it.next();
                }
            }
        } else if c != '\r' && c != '\u{8}' && c != '\u{7}' {
            out.push(c);
        }
    }
    out
}

/// Blocking: connect (once) and clone the shared client. Run on a background
/// executor.
fn ensure_client(cell: &SharedClient) -> Option<Arc<Client>> {
    let mut guard = cell.lock().ok()?;
    let dead = guard.as_ref().is_none_or(|c| !c.is_alive());
    if dead {
        *guard = Client::connect();
    }
    guard.clone()
}

impl Native {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let composer = cx.new(|cx| {
                InputState::new(window, cx).placeholder("")
            });
            cx.subscribe_in(
                &composer,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    match event {
                        InputEvent::PressEnter { shift, .. } => {
                            if !shift {
                                this.send(window, cx);
                            }
                        }
                        InputEvent::Change => this.on_composer_change(cx),
                        _ => {}
                    }
                },
            )
            .detach();
            let split_composer = cx.new(|cx| {
                InputState::new(window, cx).placeholder("")
            });
            cx.subscribe_in(
                &split_composer,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    if let InputEvent::PressEnter { shift, .. } = event {
                        if !shift {
                            this.send_split(window, cx);
                        }
                    }
                },
            )
            .detach();
            let search = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search threads")
            });
            cx.subscribe_in(
                &search,
                window,
                |this: &mut Self, _state, event, _window, cx| {
                    if let InputEvent::Change = event {
                        this.on_search_change(cx);
                    }
                },
            )
            .detach();
            let settings_search = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search settings...")
            });
            cx.subscribe_in(
                &settings_search,
                window,
                |this: &mut Self, _state, event, window, cx| match event {
                    InputEvent::Change => cx.notify(),
                    InputEvent::PressEnter { .. } => {
                        let q = this.settings_search.read(cx).value().to_lowercase();
                        if let Some(sid) = SETTINGS_SEARCH_ORDER
                            .iter()
                            .find(|(_, l)| l.to_lowercase().contains(q.trim()))
                            .map(|(id, _)| *id)
                        {
                            this.settings_section = sid;
                            if this.settings_data.is_null() {
                                this.fetch_settings(cx);
                            }
                            this.settings_search.update(cx, |s, cx| s.set_value("", window, cx));
                        }
                        cx.notify();
                    }
                    _ => {}
                },
            )
            .detach();
            let file_search = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search files")
            });
            cx.subscribe_in(
                &file_search,
                window,
                |this: &mut Self, _state, event, window, cx| match event {
                    InputEvent::Change => this.on_file_search_change(cx),
                    InputEvent::PressEnter { .. } => {
                        // Upstream: Enter picks the top match.
                        if let Some(top) = this.file_search_results.first().cloned() {
                            let dir = this.files_dir.clone();
                            let rel = str_at(&top, "path").to_string();
                            let abs = if rel.starts_with('/') {
                                rel
                            } else {
                                format!("{dir}/{rel}")
                            };
                            let mut entry = top;
                            if let Some(m) = entry.as_object_mut() {
                                m.insert("path".into(), json!(abs));
                            }
                            this.file_search.update(cx, |s, cx| s.set_value("", window, cx));
                            this.file_search_results.clear();
                            this.open_entry(entry, cx);
                        }
                    }
                    _ => {}
                },
            )
            .detach();
            cx.bind_keys([
                KeyBinding::new("cmd-f", ToggleFind, Some("native")),
                KeyBinding::new("cmd-j", ToggleTerminal, Some("native")),
            ]);
            let this = Self {
                focus_handle: cx.focus_handle(),
                client: Arc::new(Mutex::new(None)),
                snapshot: None,
                detail: None,
                selected_thread: None,
                thread_stream_id: None,
                composer,
                search,
                settings_search,
                file_search,
                file_search_results: Vec::new(),
                file_search_seq: 0,
                sending: false,
                view_files: false,
                files_dir: String::new(),
                files: Vec::new(),
                tree_entries: std::collections::HashMap::new(),
                expanded_dirs: std::collections::HashSet::new(),
                tree_loading: std::collections::HashSet::new(),
                mention_dir: String::new(),
                mention_entries: Vec::new(),
                mention_seq: 0,
                open_file: None,
                open_file_meta: None,
                file_editor: None,
                editor_synced: None,
                editing_file: false,
                file_dirty: false,
                open_in_menu: false,
                files_loading: false,
                git_status: None,
                file_stream_id: None,
                search_query: String::new(),
                search_hits: Vec::new(),
                view_dev: false,
                dev_servers: Vec::new(),
                dev_cmd: cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Dev server command")
                }),
                runtime_mode: "full-access",
                chats_collapsed: true,
                collapsed_projects: std::collections::HashSet::new(),
                view_settings: false,
                settings_section: "general",
                view_inbox: false,
                view_tasks: false,
                view_kanban: false,
                view_pullreq: false,
                view_autom: false,
                view_studio: false,
                view_spaces: false,
                inbox_items: Vec::new(),
                inbox_errors: Vec::new(),
                inbox_loading: false,
                inbox_tasks: Vec::new(),
                inbox_recap: None,
                inbox_usage: Vec::new(),
                inbox_signed_in: None,
                inbox_updates: Vec::new(),
                tools_expanded: false,
                pending_attachments: Vec::new(),
                interaction_mode: "default",
                fast_mode: false,
                queued_turns: Vec::new(),
                queue_paused: false,
                composer_thread_id: None,
                uploading: false,
                open_threads: Vec::new(),
                pinned_threads: Vec::new(),
            tab_drop_target: None,
            composer_thread_over: false,
            transcript_scroll: ScrollHandle::new(),
            editing_message_id: None,
            pending_revert: None,
    appearance_mode: 0,
    ui_density: 1,
    chat_width_mode: 0,
    show_ascii_bg: true,
            ascii_t0: std::time::Instant::now(),
            ascii_seed: 0.0,
            ascii_cells: Vec::new(),
            ascii_ripples: Vec::new(),
            ascii_fresh: true,
            split_open: false,
            split_thread: None,
            split_detail: None,
            split_stream_id: None,
            focused_pane: 0,
            split_ratio: 0.5,
            split_dragging: false,
            split_composer: Some(split_composer),
                renaming: false,
                rename_input: None,
                view_changes: false,
                staged_files: Vec::new(),
                unstaged_files: Vec::new(),
                changes_diff: None,
                blame_pop: None,
                changes_loading: false,
                changes_unavailable: false,
                dock_tab: 0,
                view_terminal: false,
                term_active: false,
                term_tabs: Vec::new(),
                term_sel: 0,
                term_input: None,
                term_events: false,
                term_pending_cwd: None,
                term_search_input: None,
                term_search_query: String::new(),
                settings_scroll: ScrollHandle::new(),
                settings_nav_scroll: ScrollHandle::new(),
                dock_tabs_scroll: ScrollHandle::new(),
                settings_data: Value::Null,
                settings_loading: false,
                dismissed_status: std::collections::HashMap::new(),
                visited_threads: std::collections::HashMap::new(),
                keep_awake_mode: 2,
                coachmarks_seen: std::collections::HashSet::new(),
                feature_flags: [false; 3],
                flags_menu_open: false,
                coachmark: None,
                sponsor_dismissed: false,
            search_open: false,
            sidebar_collapsed: false,
            user_input_open: Vec::new(),
            ctx_menu: None,
            ctx_sub: None,
            file_ctx_menu: None,
            hover_thread: None,
            hover_card: None,
            hover_card_seq: 0,
            ctx_confirm_delete: None,
            proj_ctx_menu: None,
            proj_ctx_confirm: false,
            hover_project: None,
            project_card: None,
            project_card_seq: 0,
            proj_rename: None,
            proj_rename_input: None,
            palette_mode: None,
            palette_input: None,
            palette_rows: Vec::new(),
            palette_sel: 0,
            palette_seq: 0,
            kb_rules: Vec::new(),
            sc_rec: None,
            feedback_open: false,
            feedback_category: None,
            feedback_input: None,
            feedback_sending: false,
            feedback_toast: None,
            sc_dialog_open: false,
            sc_dialog_input: None,
            autom_defs: Vec::new(),
            autom_runs: Vec::new(),
            autom_sel: None,
            autom_dialog: None,
            git_dialog: None,
            git_msg_input: None,
            git_pr_title_input: None,
            git_pr_body_input: None,
            git_pr_draft: false,
            git_commit_action: "commit".to_string(),
            git_action_running: false,
            tour_step: None,
            user_input_draft: Vec::new(),
            find_open: false,
                find_input: None,
                find_query: String::new(),
            };
            this.refresh(cx);
            // Give the root view initial focus so global chords dispatch even
            // before an input takes focus (chords bubble from inputs anyway).
            this.focus_handle.focus(window, cx);
            // AsciiWaveBackground ambient clock: ~15fps repaints while the
            // landing field is on screen (upstream AMBIENT_FRAME_INTERVAL_MS).
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(66))
                        .await;
                    let keep = this
                        .update(cx, |this: &mut Self, cx| {
                            if this.show_ascii_bg && this.landing_active() {
                                cx.notify();
                            }
                        })
                        .is_ok();
                    if !keep {
                        break;
                    }
                }
            })
            .detach();
            this
        })
    }

    /// The new-chat landing is the transcript pane when no rail surface is
    /// open and the selected thread is empty (mirrors the render branch).
    fn landing_active(&self) -> bool {
        if self.view_settings
            || self.view_dev
            || self.view_files
            || self.view_inbox
            || self.view_tasks
            || self.view_kanban
            || self.view_pullreq
            || self.view_autom
            || self.view_studio
            || self.view_changes
        {
            return false;
        }
        if self.view_terminal && self.term_active {
            return false;
        }
        if self.selected_thread.is_none() {
            return true;
        }
        arr(
            get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
            "messages",
        )
        .is_empty()
    }

    /// (Re)fetch the shell snapshot, then keep it live via subscribeShell.
    fn refresh(&self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                let cell = client.clone();
                let client = cx
                    .background_executor()
                    .spawn(async move { ensure_client(&cell) })
                    .await;
                let Some(client) = client else {
                    eprintln!("native: ws connect failed");
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(3))
                        .await;
                    continue;
                };
                let c = client.clone();
                let snapshot = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                    .await;
                let c2 = client.clone();
                let cfg = cx
                    .background_executor()
                    .spawn(async move { c2.call("server.getConfig", json!({})) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(rules) = cfg.and_then(|v| {
                        v.get("keybindings")
                            .and_then(|k| k.as_array())
                            .cloned()
                    }) {
                        this.kb_rules = rules;
                    }
                    if let Some(s) = snapshot {
                        this.snapshot = Some(s);
                    }
                    this.fetch_git_status(cx);
                    if this.settings_data.is_null() {
                        this.fetch_settings(cx);
                    }
                    cx.notify();
                });
                // Live updates: subscribeShell streams shell-domain events; on
                // each chunk just refetch the snapshot.
                let (tx, rx) = async_channel::unbounded::<()>();
                let c = client.clone();
                cx.background_executor()
                    .spawn(async move {
                        c.call_stream("orchestration.subscribeShell", json!({}), move |_| {
                            let _ = tx.send_blocking(());
                        });
                    })
                    .detach();
                while rx.recv().await.is_ok() {
                    let c = client.clone();
                    let s = cx
                        .background_executor()
                        .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                        .await;
                    let _ = this.update(cx, |this: &mut Self, cx| {
                        if let Some(s) = s {
                            this.snapshot = Some(s);
                        }
                        cx.notify();
                    });
                }
                // The stream ended (socket dropped): reconnect and resubscribe.
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    fn open_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        // Close the previous thread stream if any.
        let old = self.thread_stream_id.take();
        self.selected_thread = Some(thread_id.clone());
        // Upstream markThreadVisited: lastVisitedAt is monotonic.
        {
            let prev = self
                .visited_threads
                .get(&thread_id)
                .cloned()
                .unwrap_or_default();
            let now = now_iso();
            if now > prev {
                self.visited_threads.insert(thread_id.clone(), now);
                self.persist_sidebar_ui(cx);
            }
        }
        self.renaming = false;
        // Selecting a thread always returns to the transcript surface.
        self.clear_views();
        self.fetch_git_status(cx);
        if !self.open_threads.contains(&thread_id) {
            self.open_threads.push(thread_id.clone());
        }
        self.detail = None;
        let client = self.client.clone();
        let tid = thread_id;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            if let Some(old) = old {
                client.interrupt(&old);
            }
            let payload = json!({"threadId": tid});
            let c = client.clone();
            let p = payload.clone();
            let detail = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.selected_thread.as_deref() == Some(tid.as_str()) {
                    this.detail = detail;
                    this.transcript_scroll.scroll_to_bottom();
                }
                cx.notify();
            });
            // Live transcript via subscribeThread: each chunk triggers a detail
            // refetch (chunk shapes vary; refetch is schema-free).
            let c = client.clone();
            let (rid, frames) = match c.request("orchestration.subscribeThread", payload.clone()) {
                Some(v) => v,
                None => return,
            };
            let rid_for_ui = rid.clone();
            let _ = this.update(cx, |this: &mut Self, _| {
                this.thread_stream_id = Some(rid_for_ui);
            });
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                    c.finish_request(&rid);
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let p = payload.clone();
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if this.selected_thread.as_deref() == Some(tid.as_str()) {
                        // Keep end-follow while the reader is at the bottom
                        // (upstream tail anchor); releasing upward keeps the
                        // position and shows the scroll-to-bottom button.
                        let off = this.transcript_scroll.offset();
                        let max = this.transcript_scroll.max_offset();
                        let at_bottom = max.y + off.y <= px(120.);
                        this.detail = d;
                        if at_bottom {
                            this.transcript_scroll.scroll_to_bottom();
                        }
                    }
                    cx.notify();
                });
                // Queued follow-ups drain once the turn settles (upstream's
                // queuedComposerDrain; a user Stop holds them instead).
                let next = this
                    .update(cx, |this: &mut Self, _| {
                        if this.queue_paused
                            || this.sending
                            || this.queued_turns.is_empty()
                            || session_phase(this.detail.as_ref()) != "ready"
                        {
                            return None;
                        }
                        this.selected_thread.clone().map(|t| {
                            (
                                t,
                                this.queued_turns.first().cloned().unwrap_or_default(),
                                this.runtime_mode,
                                this.interaction_mode,
                            )
                        })
                    })
                    .ok()
                    .flatten();
                if let Some((qtid, qtext, qmode, qimode)) = next {
                    let cmd = json!({
                        "type": "thread.turn.start",
                        "commandId": uuid(),
                        "threadId": qtid,
                        "message": {
                            "messageId": uuid(),
                            "role": "user",
                            "text": qtext,
                            "attachments": [],
                        },
                        "runtimeMode": qmode,
                        "interactionMode": qimode,
                        "createdAt": now_iso(),
                    });
                    let c = client.clone();
                    let r = cx
                        .background_executor()
                        .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                        .await;
                    if r.is_some() {
                        let _ = this.update(cx, |this: &mut Self, cx| {
                            if !this.queued_turns.is_empty() {
                                this.queued_turns.remove(0);
                            }
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
        cx.notify();
    }

    /// Deselect the current thread so the composer starts a new one.
    fn new_thread(&mut self, cx: &mut Context<Self>) {
        if let Some(old) = self.thread_stream_id.take() {
            if let Ok(guard) = self.client.lock() {
                if let Some(client) = guard.as_ref() {
                    client.interrupt(&old);
                }
            }
        }
        self.selected_thread = None;
        self.detail = None;
        self.renaming = false;
        cx.notify();
    }

    /// Close one open tab; if it was selected, open the last remaining tab.
    fn close_tab(&mut self, thread_id: String, cx: &mut Context<Self>) {
        self.open_threads.retain(|t| t != &thread_id);
        if self.selected_thread.as_deref() == Some(thread_id.as_str()) {
            match self.open_threads.last().cloned() {
                Some(next) => self.open_thread(next, cx),
                None => self.new_thread(cx),
            }
        } else {
            cx.notify();
        }
    }

    fn toggle_pin(&mut self, thread_id: String, cx: &mut Context<Self>) {
        if self.pinned_threads.contains(&thread_id) {
            self.pinned_threads.retain(|t| t != &thread_id);
        } else {
            self.pinned_threads.push(thread_id);
        }
        cx.notify();
    }

    fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = str_at(
            get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
            "title",
        )
        .to_string();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Thread title"));
        input.update(cx, |s, cx| {
            s.set_value(title, window, cx);
        });
        cx.subscribe_in(&input, window, |this: &mut Self, _s, event, _w, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.commit_rename(cx);
            }
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.rename_input = Some(input);
        self.renaming = true;
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.rename_input.take() else {
            return;
        };
        let title = input.read(cx).value().trim().to_string();
        self.renaming = false;
        if title.is_empty() {
            cx.notify();
            return;
        }
        let Some(tid) = self.selected_thread.clone() else {
            cx.notify();
            return;
        };
        if let Some(detail) = self.detail.as_mut() {
            if let Some(t) = detail.get_mut("thread") {
                t["title"] = json!(title);
            }
        }
        let client = self.client.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.meta.update",
                "commandId": uuid(),
                "threadId": tid,
                "title": title,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        cx.notify();
    }

    /// Upstream toggleProjectPinned: flips isPinned via project.meta.update.
    fn toggle_project_pin(&mut self, project_id: String, pinned: bool, cx: &mut Context<Self>) {
        self.dispatch_cmd(
            json!({
                "type": "project.meta.update",
                "commandId": uuid(),
                "projectId": project_id,
                "isPinned": !pinned,
            }),
            cx,
        );
    }

    /// Upstream archiveAllThreadsInProject / deleteProjectThreads: one
    /// command per live thread filed under the project.
    fn project_threads_cmd(&mut self, project_id: String, kind: &str, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let ids: Vec<String> = arr(snapshot, "threads")
            .iter()
            .filter(|t| {
                str_at(t, "projectId") == project_id
                    && get(t, "deletedAt").is_null()
                    && get(t, "archivedAt").is_null()
            })
            .map(|t| str_at(t, "id").to_string())
            .collect();
        for tid in ids {
            self.dispatch_cmd(
                json!({
                    "type": kind,
                    "commandId": uuid(),
                    "threadId": tid,
                }),
                cx,
            );
        }
    }

    /// Upstream removeProject: threads are deleted first (project.delete
    /// refuses non-empty folders), then the project row itself.
    fn delete_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        self.project_threads_cmd(project_id.clone(), "thread.delete", cx);
        self.dispatch_cmd(
            json!({
                "type": "project.delete",
                "commandId": uuid(),
                "projectId": project_id,
            }),
            cx,
        );
    }

    /// Shared RenameDialog for projects (the "Edit project" menu action).
    fn start_project_rename(
        &mut self,
        project_id: String,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Project name"));
        input.update(cx, |s, cx| {
            s.set_value(current, window, cx);
        });
        cx.subscribe_in(&input, window, |this: &mut Self, _s, event, _w, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.commit_project_rename(cx);
            }
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.proj_rename_input = Some(input);
        self.proj_rename = Some(project_id);
        cx.notify();
    }

    fn commit_project_rename(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.proj_rename_input.take() else {
            return;
        };
        let title = input.read(cx).value().trim().to_string();
        let Some(pid) = self.proj_rename.take() else {
            return;
        };
        if title.is_empty() {
            cx.notify();
            return;
        }
        self.dispatch_cmd(
            json!({
                "type": "project.meta.update",
                "commandId": uuid(),
                "projectId": pid,
                "title": title,
            }),
            cx,
        );
        cx.notify();
    }

    /// Workspace root of the first project that still exists on disk — the
    /// file browser starts there. Seeded/stale projects are skipped.
    fn files_root(&self) -> Option<String> {
        for p in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects") {
            let r = str_at(p, "workspaceRoot");
            let root = if r.is_empty() {
                str_at(p, "workspace_root")
            } else {
                r
            };
            if !root.is_empty() && std::path::Path::new(root).is_dir() {
                return Some(root.to_string());
            }
        }
        None
    }

    fn toggle_files(&mut self, cx: &mut Context<Self>) {
        self.view_files = !self.view_files;
        if self.view_files {
            self.view_dev = false;
            self.view_inbox = false;
            self.view_tasks = false;
            self.view_kanban = false;
            self.view_pullreq = false;
            self.view_autom = false;
            self.view_studio = false;
            self.view_spaces = false;
            self.view_settings = false;
            if self.files_dir.is_empty() {
                if let Some(root) = self.files_root() {
                    self.load_dir(root, cx);
                }
            }
        }
        cx.notify();
    }

    fn toggle_dev(&mut self, cx: &mut Context<Self>) {
        self.view_dev = !self.view_dev;
        if self.view_dev {
            self.view_files = false;
            self.view_inbox = false;
            self.fetch_dev_servers(cx);
        }
        cx.notify();
    }

    fn toggle_inbox(&mut self, cx: &mut Context<Self>) {
        self.view_inbox = !self.view_inbox;
        if self.view_inbox {
            self.view_files = false;
            self.view_dev = false;
            self.view_settings = false;
            self.view_tasks = false;
            self.view_kanban = false;
            self.view_pullreq = false;
            self.view_autom = false;
            self.view_studio = false;
            self.view_spaces = false;
            self.fetch_inbox(cx);
        }
        cx.notify();
    }

    /// Clear every rail-route surface (the transcript shows when all are
    /// off). Docks (changes/terminal) are independent surfaces, untouched.
    fn clear_views(&mut self) {
        self.view_files = false;
        self.view_dev = false;
        self.view_inbox = false;
        self.view_settings = false;
        self.view_tasks = false;
        self.view_kanban = false;
        self.view_pullreq = false;
        self.view_autom = false;
        self.view_studio = false;
        self.view_spaces = false;
    }

    /// Rail route toggle: open that surface (closing the others); clicking
    /// the active one again returns to the transcript.
    fn open_route(&mut self, which: u8, cx: &mut Context<Self>) {
        let was = match which {
            1 => self.view_tasks,
            2 => self.view_kanban,
            3 => self.view_pullreq,
            4 => self.view_autom,
            5 => self.view_studio,
            6 => self.view_spaces,
            _ => false,
        };
        self.clear_views();
        if !was {
            match which {
                1 => self.view_tasks = true,
                2 => self.view_kanban = true,
                3 => self.view_pullreq = true,
                4 => self.view_autom = true,
                5 => self.view_studio = true,
                6 => self.view_spaces = true,
                _ => {}
            }
        }
        if self.view_autom {
            self.fetch_automations(cx);
        }
        cx.notify();
    }

    fn toggle_changes(&mut self, cx: &mut Context<Self>) {
        self.view_changes = !self.view_changes;
        if self.view_changes {
            self.fetch_changes(cx);
        }
        cx.notify();
    }

    fn fetch_changes(&mut self, cx: &mut Context<Self>) {
        let Some(cwd) = self.files_root() else {
            self.changes_unavailable = true;
            cx.notify();
            return;
        };
        self.changes_loading = true;
        self.changes_unavailable = false;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            let c2 = cwd.clone();
            let res = cx
                .background_executor()
                .spawn(async move { c.call("git.status", json!({"cwd": c2})) })
                .await;
            let c = client.clone();
            let c2 = cwd.clone();
            let staged_res = cx
                .background_executor()
                .spawn(async move {
                    c.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": c2, "scope": "staged"}),
                    )
                })
                .await;
            let c2 = cwd.clone();
            let unstaged_res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": c2, "scope": "unstaged"}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.changes_loading = false;
                match res {
                    Some(_v) => {
                        this.changes_unavailable = false;
                        let parse = |r: Option<Value>| -> Vec<Value> {
                            r.map(|v| {
                                parse_patch_files(str_at(&v, "patch"))
                            })
                            .unwrap_or_default()
                        };
                        this.staged_files = parse(staged_res);
                        this.unstaged_files = parse(unstaged_res);
                    }
                    None => {
                        this.changes_unavailable = true;
                        this.staged_files = Vec::new();
                        this.unstaged_files = Vec::new();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn stage_paths(&mut self, paths: Vec<String>, unstage: bool, cx: &mut Context<Self>) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let method = if unstage { "git.unstageFiles" } else { "git.stageFiles" };
            let res = cx
                .background_executor()
                .spawn(async move { client.call(method, json!({"cwd": cwd, "paths": paths})) })
                .await;
            let _ = res;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_changes(cx);
            });
        })
        .detach();
    }

    fn open_diff(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let p = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": cwd, "filePath": p}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    this.blame_pop = None;
                    this.changes_diff = Some((path.clone(), str_at(&v, "patch").to_string()));
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Cmd+F: transcript find-bar. Highlights matching messages with an accent
    /// bar and shows the match count, like the web find-in-thread bar.
    /// Click on a diff line -> `git.blameLine` then the blame popover
    /// (upstream DiffLineBlamePopover). Deleted lines blame the base
    /// tree (base:"branch"); everything else blames the working tree.
    fn fetch_blame(
        &mut self,
        path: String,
        line: u32,
        base: bool,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                return;
            };
            let mut p = json!({"cwd": cwd, "filePath": path, "line": line});
            if base {
                p["base"] = json!("branch");
            }
            let res = cx
                .background_executor()
                .spawn(async move { client.call("git.blameLine", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    this.blame_pop = Some((v, x, y));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn toggle_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = !self.find_open;
        if self.find_open {
            let input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Find in thread")
            });
            cx.subscribe_in(&input, window, |this: &mut Self, state, ev, _w, cx| {
                if let InputEvent::Change = ev {
                    this.find_query = state.read(cx).value().to_string();
                    cx.notify();
                }
            })
            .detach();
            input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            self.find_input = Some(input);
        } else {
            self.find_input = None;
            self.find_query.clear();
        }
        cx.notify();
    }

    /// Open the workspace search palette: "search" = ⌘K chats+projects
    /// (upstream SidebarSearchPalette), "files" = ⌘P projects.searchEntries,
    /// "snippets" = ⇧⌘F projects.searchContent (WorkspaceSearchPalette).
    fn open_palette(&mut self, mode: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette_mode == Some(mode) && mode != "search" {
            return;
        }
        self.palette_mode = Some(mode);
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        self.palette_seq += 1;
        let placeholder = match mode {
            "files" => "Search files",
            "snippets" => "Search code",
            _ => "Search chats or run a command",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, _w, cx| match ev {
            InputEvent::Change => this.palette_query_changed(cx),
            InputEvent::PressEnter { .. } => this.palette_activate(_w, cx),
            _ => {}
        })
        .detach();
        // Focus on the next frame: during key dispatch gpui can restore focus
        // to the dispatch target, swallowing an immediate focus() call.
        window.defer(cx, {
            let input = input.clone();
            move |window, cx| {
                input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            }
        });
        self.palette_input = Some(input);
        if mode != "search" {
            // Prewarm like upstream so the first keystroke hits a warm index.
            if let Some(cwd) = self.files_root() {
                let client = self.client.clone();
                cx.background_executor()
                    .spawn(async move {
                        if let Some(c) = ensure_client(&client) {
                            c.call("projects.prewarmSearchIndex", json!({"cwd": cwd}));
                        }
                    })
                    .detach();
            }
        }
        cx.notify();
    }

    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_mode = None;
        self.palette_input = None;
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        // The input's focus handle unmounts with the popup; give focus back
        // to the root or subsequent key events dispatch to a dead handle.
        let f = self.focus_handle.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    /// Re-query on input change: RPC rows for files/snippets, a notify for
    /// chats mode (its rows are computed from the snapshot at render).
    fn palette_query_changed(&mut self, cx: &mut Context<Self>) {
        let Some(mode) = self.palette_mode else {
            return;
        };
        self.palette_sel = 0;
        if mode == "search" {
            cx.notify();
            return;
        }
        let Some(input) = self.palette_input.clone() else {
            return;
        };
        let query = input.read(cx).value().to_string();
        let min = if mode == "files" { 1 } else { 2 };
        if query.trim().len() < min {
            self.palette_rows = Vec::new();
            self.palette_seq += 1;
            cx.notify();
            return;
        }
        let Some(cwd) = self.files_root() else {
            return;
        };
        self.palette_seq += 1;
        let seq = self.palette_seq;
        let tag = if mode == "files" {
            "projects.searchEntries"
        } else {
            "projects.searchContent"
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(tag, json!({"cwd": cwd, "query": query, "limit": 30}))
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.palette_seq != seq {
                    return;
                }
                let key = if this.palette_mode == Some("files") {
                    "entries"
                } else {
                    "matches"
                };
                this.palette_rows = v.map(|v| arr(&v, key).to_vec()).unwrap_or_default();
                if this.palette_sel >= this.palette_rows.len() {
                    this.palette_sel = 0;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Current palette rows as (kind, label, sub, payload): kind picks the
    /// row icon and the activate action; file/directory/snippet payloads are
    /// cwd-relative paths, project/thread payloads are ids.
    fn palette_items(&self, query: &str) -> Vec<(String, String, String, String)> {
        let q = query.trim().to_lowercase();
        match self.palette_mode {
            Some("files") => self
                .palette_rows
                .iter()
                .map(|e| {
                    let rel = str_at(e, "path").to_string();
                    let base = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                    (
                        str_at(e, "kind").to_string(),
                        base,
                        str_at(e, "parentPath").to_string(),
                        rel,
                    )
                })
                .collect(),
            Some("snippets") => self
                .palette_rows
                .iter()
                .map(|m| {
                    let rel = str_at(m, "path").to_string();
                    let line = get(m, "lineNumber").as_i64().unwrap_or(0);
                    (
                        "snippet".to_string(),
                        format!("{rel}:{line}"),
                        str_at(m, "lineText").trim().to_string(),
                        rel,
                    )
                })
                .collect(),
            _ => {
                let snap = self.snapshot.as_ref().unwrap_or(&Value::Null);
                let mut items: Vec<(String, String, String, String)> = Vec::new();
                for p in arr(snap, "projects") {
                    let title = str_at(p, "title").to_string();
                    if !q.is_empty() && !title.to_lowercase().contains(&q) {
                        continue;
                    }
                    items.push((
                        "project".to_string(),
                        title,
                        str_at(p, "kind").to_string(),
                        str_at(p, "id").to_string(),
                    ));
                }
                for t in arr(snap, "threads") {
                    if !(get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null()) {
                        continue;
                    }
                    let title = str_at(t, "title").to_string();
                    if !q.is_empty()
                        && !title.to_lowercase().contains(&q)
                        && !str_at(t, "latestReplyPreview").to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    items.push((
                        "thread".to_string(),
                        title,
                        String::new(),
                        str_at(t, "id").to_string(),
                    ));
                }
                items.truncate(30);
                items
            }
        }
    }

    /// Activate the highlighted row: file/snippet rows open the file viewer,
    /// directories open the explorer there, projects open their workspace
    /// explorer, threads open their transcript.
    fn palette_activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self
            .palette_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        let items = self.palette_items(&query);
        let Some((kind, _label, _sub, payload)) = items.get(self.palette_sel).cloned() else {
            self.close_palette(window, cx);
            return;
        };
        let cwd = self.files_root().unwrap_or_default();
        self.close_palette(window, cx);
        match kind.as_str() {
            "directory" => {
                self.clear_views();
                self.view_files = true;
                self.load_dir(format!("{cwd}/{payload}"), cx);
            }
            "file" | "snippet" => {
                self.clear_views();
                self.view_files = true;
                self.open_entry(
                    json!({"kind": "file", "path": format!("{cwd}/{payload}")}),
                    cx,
                );
            }
            "project" => {
                self.clear_views();
                self.view_files = true;
                let root = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == payload)
                    .map(|p| {
                        let r = str_at(p, "workspaceRoot");
                        if r.is_empty() {
                            str_at(p, "workspace_root").to_string()
                        } else {
                            r.to_string()
                        }
                    })
                    .filter(|r: &String| !r.is_empty());
                if let Some(root) = root {
                    self.load_dir(root, cx);
                }
            }
            "thread" => self.open_thread(payload, cx),
            _ => {}
        }
        cx.notify();
    }

    /// Global keymap: the upstream DEFAULT_SHORTCUT_FALLBACKS chords whose
    /// targets exist on the native surface. cmd-f / cmd-j stay declared
    /// KeyBindings (chat.find / terminal.toggle); the rest is dispatched here.
    // ---- shortcut recorder + keybinding dispatch ------------------------

    /// Open the recorder dialog for a shortcut row. `replacing` holds the
    /// resolved rules of the binding being edited (empty when adding one).
    fn sc_open(
        &mut self,
        label: String,
        commands: Vec<String>,
        numbered: bool,
        replacing: Vec<Value>,
        cx: &mut Context<Self>,
    ) {
        self.sc_rec = Some(ScRec {
            label,
            commands,
            numbered,
            replacing,
            captured: None,
        });
        cx.notify();
    }

    /// `evaluateRecordedShortcut`: what saving `captured` would do.
    /// Returns (problem_message, conflicts) — empty problem + empty
    /// conflicts means "{chord} is available."
    fn sc_evaluate(&self, rec: &ScRec) -> (Option<String>, Vec<(String, Value)>) {
        let Some(cap) = &rec.captured else {
            return (None, Vec::new());
        };
        let label = sc_fmt_captured(cap);
        if rec.numbered
            && !(cap.key.len() == 1
                && cap.key.as_bytes()[0].is_ascii_digit()
                && cap.key != "0")
        {
            return (
                Some("Hold the modifiers and press a number key.".to_string()),
                Vec::new(),
            );
        }
        if !(cap.m_mod || cap.m_ctrl || cap.m_alt) && !sc_is_fkey(&cap.key) {
            return (
                Some(
                    "Add \u{2318}, \u{2325} or \u{2303} to that key, or use an F-key on its own."
                        .to_string(),
                ),
                Vec::new(),
            );
        }
        let enc = sc_enc_captured(cap);
        let mut conflicts: Vec<(String, Value)> = Vec::new();
        for r in &self.kb_rules {
            if rec
                .replacing
                .iter()
                .any(|rep| std::ptr::eq(rep, r) || rep == r)
            {
                continue;
            }
            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
            if sc_enc_shortcut(&sc) != enc {
                continue;
            }
            let rc = r
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if rec.commands.iter().any(|c| *c == rc) {
                return (
                    Some(format!("{label} is already a shortcut for this command.")),
                    Vec::new(),
                );
            }
            if !conflicts.iter().any(|(c, _)| *c == rc) {
                conflicts.push((rc, r.clone()));
            }
        }
        (None, conflicts)
    }

    /// Command label for a conflict message: the row title, else the raw
    /// command id.
    fn sc_command_label(cmd: &str) -> String {
        for (title, _d, _k, commands) in &SHORTCUT_ROWS {
            if commands.split(' ').any(|c| c == cmd) {
                return title.to_string();
            }
        }
        cmd.to_string()
    }

    /// `shortcutSaveEdits` equivalent: remove conflicts, then set the new
    /// rule(s) replacing the edited binding.
    fn sc_save(&mut self, cx: &mut Context<Self>) {
        let Some(rec) = self.sc_rec.as_ref() else {
            return;
        };
        let commands = rec.commands.clone();
        let numbered = rec.numbered;
        let replacing = rec.replacing.clone();
        let Some(cap) = rec.captured.clone() else {
            return;
        };
        let (problem, conflicts) = self.sc_evaluate(rec);
        if problem.is_some() {
            return;
        }
        let mut edits: Vec<Value> = Vec::new();
        for (_cmd, rule) in &conflicts {
            edits.push(json!({"type": "remove", "rule": sc_rule_from_resolved(rule)}));
        }
        let enc = sc_enc_captured(&cap);
        if numbered {
            // The family gets one rule per member on the captured modifiers.
            let prefix = enc
                .rsplit_once('+')
                .map(|(p, _)| format!("{p}+"))
                .unwrap_or_default();
            for (i, cmd) in commands.iter().enumerate() {
                let mut edit =
                    json!({"type": "set", "rule": {"command": cmd, "key": format!("{prefix}{}", i + 1)}});
                if let Some(rep) = replacing.iter().find(|r| {
                    r.get("command").and_then(Value::as_str) == Some(cmd.as_str())
                }) {
                    edit["replacing"] = sc_rule_from_resolved(rep);
                }
                edits.push(edit);
            }
        } else {
            for (i, cmd) in commands.iter().enumerate() {
                let mut edit =
                    json!({"type": "set", "rule": {"command": cmd, "key": enc}});
                if let Some(rep) = replacing.get(i).or_else(|| replacing.first()) {
                    edit["replacing"] = sc_rule_from_resolved(rep);
                }
                edits.push(edit);
            }
        }
        self.apply_kb_edits(edits, cx);
    }

    fn sc_remove_binding(&mut self, rules: &[Value], cx: &mut Context<Self>) {
        let edits: Vec<Value> = rules
            .iter()
            .map(|r| json!({"type": "remove", "rule": sc_rule_from_resolved(r)}))
            .collect();
        self.apply_kb_edits(edits, cx);
    }

    /// Apply a batch of keybinding edits through `server.editKeybindings`
    /// and adopt the resolved rules it returns.
    fn apply_kb_edits(&mut self, edits: Vec<Value>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                return;
            };
            let reply = cx
                .background_executor()
                .spawn(async move {
                    client.call("server.editKeybindings", json!({"edits": edits}))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(rules) = reply
                    .and_then(|v| {
                        v.get("keybindings")
                            .and_then(|k| k.as_array())
                            .cloned()
                    })
                {
                    this.kb_rules = rules.clone();
                    if !this.settings_data.is_null() {
                        if let Some(cfg) = this.settings_data.get_mut("config") {
                            cfg["keybindings"] = json!(rules);
                        }
                    }
                }
                this.sc_rec = None;
                cx.notify();
            });
        })
        .detach();
    }

    /// Route a key event while the recorder is open: Escape cancels, Enter
    /// saves, everything else is captured as the candidate chord.
    fn sc_rec_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let m = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str().to_string();
        match key.as_str() {
            "escape" => {
                self.sc_rec = None;
                cx.notify();
            }
            "enter" => self.sc_save(cx),
            "shift" | "control" | "alt" | "meta" | "caps_lock" | "tab" | "function" => {}
            _ => {
                if let Some(r) = &mut self.sc_rec {
                    r.captured = Some(ScCaptured {
                        key,
                        m_mod: m.platform,
                        m_ctrl: m.control,
                        m_alt: m.alt,
                        m_shift: m.shift,
                    });
                }
                cx.notify();
            }
        }
    }

    /// Resolve a key event to a command using the resolved keybinding
    /// rules (mac: `mod`/`meta` both mean the platform ⌘ key).
    fn kb_command_for_event(&self, event: &KeyDownEvent) -> Option<String> {
        let m = event.keystroke.modifiers;
        let key = sc_norm_rule_key(&event.keystroke.key);
        for r in &self.kb_rules {
            if !sc_when_ok(r.get("when").and_then(Value::as_str)) {
                continue;
            }
            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
            let cmd_key = sc.get("key").and_then(Value::as_str).unwrap_or("");
            if cmd_key.is_empty() || sc_norm_rule_key(cmd_key) != key {
                continue;
            }
            if (sc_flag(&sc, "modKey") || sc_flag(&sc, "metaKey")) != m.platform {
                continue;
            }
            if sc_flag(&sc, "ctrlKey") != m.control
                || sc_flag(&sc, "altKey") != m.alt
                || sc_flag(&sc, "shiftKey") != m.shift
            {
                continue;
            }
            if let Some(c) = r.get("command").and_then(Value::as_str) {
                return Some(c.to_string());
            }
        }
        None
    }

    /// Run a keybinding command. Returns false for commands the shell has
    /// no action for (model pickers, editor.*, snooze, …) so the caller can
    /// try the next matching rule.
    fn run_command(&mut self, cmd: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match cmd {
            "chat.new" | "chat.newChat" | "chat.newLocal" | "chat.newClaude"
            | "chat.newCodex" | "chat.newCursor" | "chat.newLatestProject" => {
                self.new_thread(cx)
            }
            "threadTab.close" | "terminal.close" | "terminal.workspace.closeActive" => {
                if self.split_open && self.focused_pane == 1 {
                    if self.split_thread.is_some() {
                        self.split_thread = None;
                        self.split_detail = None;
                    } else {
                        self.split_open = false;
                    }
                    self.focused_pane = 0;
                    cx.notify();
                } else if let Some(t) = self.selected_thread.clone() {
                    self.close_tab(t, cx);
                }
            }
            "chat.split" => {
                if self.split_open {
                    self.close_split(cx);
                } else if self.selected_thread.is_some() {
                    self.split_open = true;
                    cx.notify();
                }
            }
            "composer.focus.toggle" => {
                let f = self.composer.read(cx).focus_handle(cx);
                window.focus(&f, cx);
            }
            "sidebar.search" => {
                if self.palette_mode == Some("search") {
                    self.close_palette(window, cx);
                } else {
                    self.open_palette("search", window, cx);
                }
            }
            "search.files" => self.open_palette("files", window, cx),
            "search.content" => self.open_palette("snippets", window, cx),
            "sidebar.toggle" => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                cx.notify();
            }
            "settings.usage" => {
                self.clear_views();
                self.view_settings = true;
                self.settings_section = "usage";
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            "chat.find" => self.toggle_find(window, cx),
            "chat.newTerminal" | "terminal.new" | "terminal.toggle"
            | "terminal.workspace.newFullWidth" | "terminal.workspace.terminal" => {
                self.toggle_terminal(window, cx)
            }
            "terminal.workspace.chat" => {
                if self.view_terminal {
                    self.toggle_terminal(window, cx);
                }
            }
            "sidebar.activity" => {
                self.clear_views();
                self.view_inbox = true;
                self.fetch_inbox(cx);
                cx.notify();
            }
            "thread.copyId" => {
                if let Some(t) = self.selected_thread.clone() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                }
            }
            "thread.archive" => self.archive_selected(cx),
            "chat.visible.previous" | "view.recent.previous" | "threadTab.previous"
            | "space.previous" => self.cycle_tab(-1, cx),
            "chat.visible.next" | "view.recent.next" | "threadTab.next"
            | "chat.attention.next" | "space.next" => self.cycle_tab(1, cx),
            "diff.toggle" | "git.commitAndPush" => self.toggle_changes(cx),
            c if c.starts_with("thread.jump.") => {
                if let Ok(i) = c["thread.jump.".len()..].parse::<usize>() {
                    if let Some(t) = self.open_threads.get(i.saturating_sub(1)).cloned() {
                        self.open_thread(t, cx);
                    }
                }
            }
            _ => return false,
        }
        true
    }

    fn handle_chord(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // ⌘S saves the open file while it is being edited (upstream
        // useCodeEditorSaveKeyDownHandler).
        if event.keystroke.modifiers.platform
            && event.keystroke.key.as_str() == "s"
            && !event.keystroke.modifiers.shift
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && self.editing_file
        {
            self.save_file(cx);
            return;
        }
        // While the recorder dialog is open every key is input for it.
        if self.sc_rec.is_some() {
            self.sc_rec_key(event, window, cx);
            return;
        }
        // Once keybinding rules are loaded they own shortcut dispatch
        // (they already include the defaults). ⌘, is not a server rule —
        // keep it hardcoded.
        if !self.kb_rules.is_empty() {
            let m = event.keystroke.modifiers;
            if m.function {
                return;
            }
            if let Some(cmd) = self.kb_command_for_event(event) {
                let _ = self.run_command(&cmd, window, cx);
            } else {
                let key = event.keystroke.key.as_str();
                if m.platform && !m.control && !m.alt && !m.shift && key == "," {
                    self.clear_views();
                    self.view_settings = true;
                    if self.settings_data.is_null() {
                        self.fetch_settings(cx);
                    }
                    cx.notify();
                } else if m.platform && !m.control && !m.alt && !m.shift && key == "/" {
                    self.open_sc_dialog(window, cx);
                }
            }
            return;
        }
        let m = event.keystroke.modifiers;
        if !m.platform || m.function {
            return;
        }
        let key = event.keystroke.key.as_str();
        match (m.control, m.alt, m.shift, key) {
            // ⌘ — chat.new / chat.newChat
            (false, false, false, "n") | (false, true, false, "n") => self.new_thread(cx),
            // ⌘W — threadTab.close (split: closes the focused pane first)
            (false, false, false, "w") => {
                if self.split_open && self.focused_pane == 1 {
                    if self.split_thread.is_some() {
                        self.split_thread = None;
                        self.split_detail = None;
                    } else {
                        self.split_open = false;
                    }
                    self.focused_pane = 0;
                    cx.notify();
                } else if let Some(t) = self.selected_thread.clone() {
                    self.close_tab(t, cx);
                }
            }
            // ⌘\ — chat.split
            (false, false, false, "\\") => {
                if self.split_open {
                    self.close_split(cx);
                } else if self.selected_thread.is_some() {
                    self.split_open = true;
                    cx.notify();
                }
            }
            // ⌘L — composer.focus.toggle
            (false, false, false, "l") => {
                let f = self.composer.read(cx).focus_handle(cx);
                window.focus(&f, cx);
            }
            // ⌘K — sidebar.search: chats+projects palette (toggles).
            (false, false, false, "k") => {
                if self.palette_mode == Some("search") {
                    self.close_palette(window, cx);
                } else {
                    self.open_palette("search", window, cx);
                }
            }
            // ⌘P — search.files: workspace file-name palette.
            (false, false, false, "p") => self.open_palette("files", window, cx),
            // ⌘/ — upstream keyboard-shortcuts-help dialog.
            (false, false, false, "/") => self.open_sc_dialog(window, cx),
            // ⌘⇧F — search.content: workspace code-snippet palette.
            (false, false, true, "f") => self.open_palette("snippets", window, cx),
            // ⌘B — sidebar collapse
            (false, false, false, "b") => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                cx.notify();
            }
            // ⌘, — settings
            (false, false, false, ",") => {
                self.clear_views();
                self.view_settings = true;
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            // ⌘1..⌘9 — thread.jump.N
            (false, false, false, d) if d.len() == 1 => {
                let i = d
                    .as_bytes()
                    .first()
                    .filter(|b| b.is_ascii_digit() && **b != b'0')
                    .map(|b| (b - b'1') as usize);
                if let Some(t) = i.and_then(|i| self.open_threads.get(i).cloned()) {
                    self.open_thread(t, cx);
                }
            }
            // ⌘⇧T / ⌘⇧J — chat.newTerminal / terminal.workspace.newFullWidth
            (false, false, true, "t") | (false, false, true, "j") => {
                self.toggle_terminal(window, cx)
            }
            // ⌘⇧U — settings.usage
            (false, false, true, "u") => {
                self.clear_views();
                self.view_settings = true;
                self.settings_section = "usage";
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            // ⌘⇧C — thread.copyId
            (false, false, true, "c") => {
                if let Some(t) = self.selected_thread.clone() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                }
            }
            // ⌘⌃→ / ⌘⌃Tab — threadTab.next / view.recent.next
            (true, false, false, "right") | (true, false, false, "tab") => {
                self.cycle_tab(1, cx)
            }
            // ⌘⌃← / ⌘⌃⇧Tab — threadTab.previous / view.recent.previous
            (true, false, false, "left") | (true, false, true, "tab") => {
                self.cycle_tab(-1, cx)
            }
            // ⌘⌃P — git.commitAndPush opens the changes pane
            (true, false, false, "p") => self.toggle_changes(cx),
            // ⌘⌥U — sidebar.activity → inbox
            (false, true, false, "u") => {
                self.clear_views();
                self.view_inbox = true;
                self.fetch_inbox(cx);
                cx.notify();
            }
            // ⌘⌥A — chat.attention.next → cycle open threads
            (false, true, false, "a") => self.cycle_tab(1, cx),
            // ⌘⌥⇧A — thread.archive
            (false, true, true, "a") => self.archive_selected(cx),
            _ => {}
        }
    }

    /// Cycle open thread tabs by offset (wraps).
    fn cycle_tab(&mut self, offset: i64, cx: &mut Context<Self>) {
        let n = self.open_threads.len();
        if n == 0 {
            return;
        }
        let cur = self
            .selected_thread
            .as_ref()
            .and_then(|s| self.open_threads.iter().position(|t| t == s))
            .unwrap_or(0) as i64;
        let next = ((cur + offset) % n as i64 + n as i64) as usize % n;
        let t = self.open_threads[next].clone();
        self.open_thread(t, cx);
    }

    /// ⌥⇧⌘A — archive the selected thread via dispatchCommand, then close
    /// its tab (the shell snapshot refresh removes it from the sidebar).
    fn archive_selected(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let tid_cmd = tid.clone();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.archive",
                "commandId": uuid(),
                "threadId": tid_cmd,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        self.close_tab(tid, cx);
    }

    /// Context-menu actions: fork or hand off a thread. Both dispatch the
    /// lineage command upstream's threadContextMenu sends, then refetch the
    /// snapshot so the new row appears in the sidebar.
    fn lineage(
        &mut self,
        tid: String,
        is_handoff: bool,
        env_mode: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        let project_id = self
            .snapshot
            .as_ref()
            .and_then(|s| {
                arr(s, "threads").iter().find_map(|t| {
                    if str_at(t, "id") == tid {
                        Some(str_at(t, "projectId").to_string())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "default".to_string());
        let client = self.client.clone();
        let ctype = if is_handoff {
            "thread.handoff.create"
        } else {
            "thread.fork.create"
        };
        cx.background_executor()
            .spawn(async move {
                let Some(client) = ensure_client(&client) else {
                    return;
                };
                let mut payload = json!({
                    "type": ctype,
                    "commandId": uuid(),
                    "threadId": uuid(),
                    "projectId": project_id,
                    "sourceThreadId": tid,
                    "createdAt": now_iso(),
                });
                if let Some(mode) = env_mode {
                    payload["envMode"] = json!(mode);
                }
                client.call("orchestration.dispatchCommand", payload);
            })
            .detach();
    }

    /// Upstream resolveThreadWorkspaceCwd: worktree threads resolve to their
    /// worktreePath (or a workingDirectory inside it); local threads resolve
    /// to workingDirectory or the project's workspaceRoot.
    fn thread_workspace_cwd(&self, tid: &str) -> Option<String> {
        let s = self.snapshot.as_ref()?;
        let t = arr(s, "threads").iter().find(|t| str_at(t, "id") == tid)?;
        let env_mode = str_at(t, "envMode");
        let wt = str_at(t, "worktreePath");
        let wd = str_at(t, "workingDirectory");
        if env_mode == "worktree" {
            if !wt.is_empty() && !wd.is_empty() && wd.starts_with(wt) {
                return Some(wd.to_string());
            }
            return (!wt.is_empty()).then(|| wt.to_string());
        }
        if !wd.is_empty() {
            return Some(wd.to_string());
        }
        let pid = str_at(t, "projectId");
        arr(s, "projects").iter().find_map(|p| {
            if str_at(p, "id") != pid {
                return None;
            }
            let r = str_at(p, "workspaceRoot");
            let r = if r.is_empty() { str_at(p, "workspace_root") } else { r };
            (!r.is_empty()).then(|| r.to_string())
        })
    }

    /// Context-menu "Open Path in Terminal": when the drawer is open send a
    /// `cd`, otherwise seed the next terminal.open with the path and open it.
    fn open_terminal_at(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_terminal {
            self.term_write(format!("cd '{}'\r", path.replace('\'', "'\\''")), cx);
        } else {
            self.term_pending_cwd = Some(path);
            self.toggle_terminal(window, cx);
        }
    }

    /// Context-menu Delete: dispatch thread.delete, unpin the row and close
    /// its tab, like upstream deleteThread.
    fn delete_thread(&mut self, tid: String, cx: &mut Context<Self>) {
        self.pinned_threads.retain(|p| p != &tid);
        let client = self.client.clone();
        let tid_cmd = tid.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.delete",
                "commandId": uuid(),
                "threadId": tid_cmd,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        self.close_tab(tid, cx);
    }

    /// Rail button: toggle the bottom terminal drawer. Opening spawns a PTY
    /// session on the workspace root via terminal.open and subscribes to its
    /// event stream; closing kills it.
    fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_terminal = !self.view_terminal;
        self.term_active = self.view_terminal;
        if self.view_terminal {
            if self.term_tabs.is_empty() {
                self.term_tabs.push(TermTab::default());
            }
            self.term_sel = self.term_tabs.len() - 1;
            if self.term_input.is_none() {
                let input = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("type a command")
                });
                cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, w, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.term_send(w, cx);
                    }
                })
                .detach();
                self.term_input = Some(input);
            }
            self.ensure_terminal_events(cx);
            self.open_terminal(cx);
        } else {
            self.close_all_terminals(cx);
        }
        cx.notify();
    }

    /// "+" chip in the terminal sub-strip: spawn a sibling PTY session.
    fn term_new_tab(&mut self, cx: &mut Context<Self>) {
        self.term_tabs.push(TermTab::default());
        self.term_sel = self.term_tabs.len() - 1;
        self.open_terminal(cx);
        cx.notify();
    }

    /// "x" on a tab: kill that PTY; when the last one dies the input row
    /// goes away (the surface itself is closed by the strip's ×).
    fn term_close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx >= self.term_tabs.len() {
            return;
        }
        let tab = self.term_tabs.remove(idx);
        if self.term_sel > idx {
            self.term_sel -= 1;
        } else if self.term_sel >= self.term_tabs.len() {
            self.term_sel = self.term_tabs.len().saturating_sub(1);
        }
        if self.term_tabs.is_empty() {
            self.term_input = None;
            self.term_search_input = None;
            self.term_search_query.clear();
        }
        let client = self.client.clone();
        let terminal_id = tab.id;
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.close",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "deleteHistory": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
        cx.notify();
    }

    fn term_select_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.term_tabs.len() {
            self.term_sel = idx;
            cx.notify();
        }
    }

    /// Strip ×: kill every PTY session (the surface itself closes via
    /// toggle_terminal, which calls this).
    fn close_all_terminals(&mut self, cx: &mut Context<Self>) {
        for i in (0..self.term_tabs.len()).rev() {
            self.term_close_tab(i, cx);
        }
    }

    /// Magnifier in the terminal sub-strip: toggle the in-buffer find bar
    /// (upstream TerminalSearch). Esc/× closes and clears the query.
    fn toggle_term_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.term_search_input.is_some() {
            self.term_search_input = None;
            self.term_search_query.clear();
        } else {
            let input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("find in buffer")
            });
            cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, _w, cx| {
                if let InputEvent::Change = ev {
                    this.term_search_query = _s.read(cx).value().to_string();
                    cx.notify();
                }
            })
            .detach();
            input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            self.term_search_input = Some(input);
        }
        cx.notify();
    }

    fn term_active(&self) -> &TermTab {
        &self.term_tabs[self.term_sel]
    }

    fn term_thread_id(&self) -> String {
        self.selected_thread
            .clone()
            .unwrap_or_else(|| "shell".to_string())
    }

    fn open_terminal(&mut self, cx: &mut Context<Self>) {
        if self.term_tabs.is_empty() {
            self.term_tabs.push(TermTab::default());
            self.term_sel = 0;
        }
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        let cwd = self
            .term_pending_cwd
            .take()
            .or_else(|| self.files_root())
            .unwrap_or_else(|| "/".to_string());
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.open",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "cwd": cwd,
                            "cols": 140,
                            "rows": 30,
                            "streamOutput": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    fn close_terminal(&mut self, cx: &mut Context<Self>) {
        self.term_input = None;
        if let Some(tab) = self.term_tabs.get_mut(self.term_sel) {
            tab.status = None;
            tab.buf.clear();
        }
        let client = self.client.clone();
        let Some(terminal_id) = self.term_tabs.get(self.term_sel).map(|t| t.id.clone()) else {
            return;
        };
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.close",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "deleteHistory": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    fn term_restart(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        let cwd = self.files_root().unwrap_or_else(|| "/".to_string());
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.restart",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "cwd": cwd,
                            "cols": 140,
                            "rows": 30,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    fn term_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.term_input.clone() else {
            return;
        };
        let text = input.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        input.update(cx, |s, cx| s.clean(window, cx));
        self.term_write(format!("{text}\r"), cx);
    }

    /// Send raw input to the open PTY session via terminal.write.
    fn term_write(&mut self, data: String, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.write",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "data": data,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    /// Long-lived subscribeEvents stream feeding the terminal drawer; it
    /// reconnects itself while the drawer is open.
    fn ensure_terminal_events(&mut self, cx: &mut Context<Self>) {
        if self.term_events {
            return;
        }
        self.term_events = true;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                let open = this
                    .update(cx, |t: &mut Self, _| t.view_terminal)
                    .unwrap_or(false);
                if !open {
                    break;
                }
                let cell = client.clone();
                let client = cx
                    .background_executor()
                    .spawn(async move { ensure_client(&cell) })
                    .await;
                let Some(client) = client else {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(3))
                        .await;
                    continue;
                };
                let (tx, rx) = async_channel::unbounded::<Vec<Value>>();
                let c = client.clone();
                cx.background_executor()
                    .spawn(async move {
                        c.call_stream("terminal.subscribeEvents", json!({}), move |v| {
                            let _ = tx.send_blocking(v);
                        });
                    })
                    .detach();
                while let Ok(values) = rx.recv().await {
                    let _ = this.update(cx, |t: &mut Self, cx| {
                        for ev in values {
                            t.on_terminal_event(ev);
                        }
                        cx.notify();
                    });
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
            }
            let _ = this.update(cx, |t: &mut Self, _| {
                t.term_events = false;
            });
        })
        .detach();
    }

    fn on_terminal_event(&mut self, ev: Value) {
        if str_at(&ev, "threadId") != self.term_thread_id() {
            return;
        }
        let id = str_at(&ev, "terminalId");
        let Some(tab) = self.term_tabs.iter_mut().find(|t| t.id == id) else {
            return;
        };
        match str_at(&ev, "type") {
            "started" | "restarted" => {
                let snap = get(&ev, "snapshot");
                tab.buf = strip_ansi(str_at(snap, "history"));
                tab.status = Some(str_at(snap, "status").to_string());
            }
            "output" => {
                tab.buf.push_str(&strip_ansi(str_at(&ev, "data")));
                if tab.buf.len() > 200_000 {
                    let mut cut = tab.buf.len() - 100_000;
                    while !tab.buf.is_char_boundary(cut) {
                        cut += 1;
                    }
                    tab.buf.drain(..cut);
                }
            }
            "cleared" => tab.buf.clear(),
            "exited" => {
                tab.status = Some("exited".to_string());
                let suffix = get(&ev, "exitCode")
                    .as_i64()
                    .map(|c| format!(" code {c}"))
                    .unwrap_or_default();
                tab.buf.push_str(&format!("\n[process exited{suffix}]\n"));
            }
            _ => {}
        }
    }

    /// "+" button: native file picker -> upload each file -> chip in composer.
    fn attach(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach files".into()),
        });
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = rx.await.ok().and_then(|r| r.ok()).flatten().unwrap_or_default();
            let cell = client.clone();
            let thread_id = thread_id.unwrap_or_default();
            let uploaded: Vec<Value> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Vec::new();
                    };
                    let mut out = Vec::new();
                    for p in paths {
                        let Ok(bytes) = std::fs::read(&p) else {
                            continue;
                        };
                        let name = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let (kind, mime) = match ext.as_str() {
                            "png" => ("image", "image/png"),
                            "jpg" | "jpeg" => ("image", "image/jpeg"),
                            "gif" => ("image", "image/gif"),
                            "webp" => ("image", "image/webp"),
                            "pdf" => ("file", "application/pdf"),
                            "txt" | "md" => ("file", "text/plain"),
                            "json" => ("file", "application/json"),
                            "zip" => ("file", "application/zip"),
                            _ => ("file", "application/octet-stream"),
                        };
                        if let Some(att) =
                            c.upload_attachment(&thread_id, kind, &name, mime, &bytes)
                        {
                            out.push(att);
                        }
                    }
                    out
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                this.pending_attachments.extend(uploaded);
                cx.notify();
            });
        })
        .detach();
    }

    /// Composer extras "+" -> Goal: upstream inserts a literal `/goal` slash
    /// command so the goal chip flow is the same as typing it.
    fn set_draft(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let composer = self.composer.clone();
        self.composer.update(cx, |state, cx| {
            state.set_value(text.to_string(), window, cx);
        });
        let focus = composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Fire-and-forget `orchestration.dispatchCommand`, like upstream's
    /// `orchestration.dispatchCommand(...).forget()` call sites.
    fn dispatch_cmd(&mut self, cmd: Value, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", cmd) })
                .detach();
        })
        .detach();
    }

    /// Upstream FEEDBACK_CATEGORIES: (value, label, summary lead).
    const FEEDBACK_CATEGORIES: [(&str, &str, &str); 6] = [
        ("bug", "Bug", "I ran into a bug"),
        ("session", "Session", "I hit a session problem"),
        ("ui", "UI", "Something looked wrong"),
        ("performance", "Performance", "Synara felt slow"),
        ("idea", "Idea", "I have an idea"),
        ("other", "Other", "I have some feedback"),
    ];

    /// Upstream ShortcutsDialog (isKeyboardShortcutsHelpShortcut, mod+/).
    fn open_sc_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search shortcuts...")
        });
        cx.subscribe_in(&input, window, |_this, _s, event, _w, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        })
        .detach();
        self.sc_dialog_input = Some(input.clone());
        self.sc_dialog_open = true;
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn git_cwd(&self) -> Option<String> {
        self.thread_project_root().or_else(|| self.files_root())
    }

    fn open_git_dialog(
        &mut self,
        kind: GitDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ph = match kind {
            GitDialog::Commit => "Commit message",
            GitDialog::Pr => "Pull request title",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(ph));
        cx.subscribe_in(&input, window, |_this, _s, event, _w, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        })
        .detach();
        match kind {
            GitDialog::Commit => {
                self.git_msg_input = Some(input.clone());
                self.git_commit_action = "commit".to_string();
            }
            GitDialog::Pr => {
                self.git_pr_title_input = Some(input.clone());
                let body = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Pull request body")
                });
                cx.subscribe_in(&body, window, |_this, _s, event, _w, cx| {
                    if let InputEvent::Change = event {
                        cx.notify();
                    }
                })
                .detach();
                self.git_pr_body_input = Some(body);
                self.git_pr_draft = false;
            }
        }
        self.git_dialog = Some(kind);
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn close_git_dialog(&mut self, cx: &mut Context<Self>) {
        self.git_dialog = None;
        cx.notify();
    }

    /// git.runStackedAction: action in commit|commit_push|push|create_pr;
    /// dialog "commit_new_branch" maps to action=commit + featureBranch=true.
    fn run_git_action(&mut self, action: &str, cx: &mut Context<Self>) {
        if self.git_action_running {
            return;
        }
        let Some(cwd) = self.git_cwd() else {
            self.feedback_toast =
                Some(("No repository for this thread.".to_string(), true));
            cx.notify();
            return;
        };
        let mut payload = json!({"cwd": cwd, "action": action});
        let mut effective = action.to_string();
        if self.git_dialog == Some(GitDialog::Commit) {
            let msg = self
                .git_msg_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            if !msg.trim().is_empty() {
                payload["commitMessage"] = json!(msg.trim());
            }
            if action == "commit_new_branch" {
                effective = "commit".to_string();
                payload["action"] = json!("commit");
                payload["featureBranch"] = json!(true);
            }
        } else if self.git_dialog == Some(GitDialog::Pr) {
            let title = self
                .git_pr_title_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            let body = self
                .git_pr_body_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            if !title.trim().is_empty() {
                payload["prTitle"] = json!(title.trim());
            }
            if !body.trim().is_empty() {
                payload["prBody"] = json!(body);
            }
            if self.git_pr_draft {
                payload["prDraft"] = json!(true);
            }
            payload["action"] = json!("create_pr");
            effective = "create_pr".to_string();
        }
        self.git_dialog = None;
        self.git_action_running = true;
        let cell = self.client.clone();
        cx.spawn(
            async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let v = cx
                    .background_executor()
                    .spawn(async move {
                        ensure_client(&cell)
                            .and_then(|c| c.call("git.runStackedAction", payload))
                    })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.git_action_running = false;
                    match &v {
                        Some(r) => {
                            let c = get(r, "commit");
                            let p = get(r, "push");
                            let pr = get(r, "pr");
                            let msg = if str_at(&pr, "status") == "created"
                                || str_at(&pr, "status") == "opened_existing"
                            {
                                let n = pr
                                    .get("number")
                                    .and_then(Value::as_u64)
                                    .map(|n| format!(" #{n}"))
                                    .unwrap_or_default();
                                format!("Created PR{n}")
                            } else if str_at(&c, "status") == "created" {
                                let sha =
                                    str_at(&c, "commitSha").chars().take(7).collect::<String>();
                                if str_at(&p, "status") == "pushed" {
                                    format!("Committed {sha} and pushed")
                                } else {
                                    format!("Committed {sha}")
                                }
                            } else if str_at(&p, "status") == "pushed" {
                                "Pushed".to_string()
                            } else if str_at(&c, "status") == "skipped_no_changes" {
                                "No changes to commit.".to_string()
                            } else {
                                "Done".to_string()
                            };
                            this.feedback_toast = Some((msg, false));
                            this.fetch_git_status(cx);
                            this.fetch_changes(cx);
                        }
                        None => {
                            this.feedback_toast = Some(("Git action failed.".to_string(), true));
                        }
                    }
                    cx.notify();
                });
                let _ = effective;
            },
        )
        .detach();
    }

    /// Effective kbd label for a SHORTCUT_ROWS entry: the resolved binding
    /// when rules loaded, else the documented default.
    fn sc_row_label(&self, commands: &str, fallback: &str) -> String {
        if !self.kb_rules.is_empty() {
            for r in &self.kb_rules {
                let rc = r.get("command").and_then(Value::as_str).unwrap_or("");
                if commands.split(' ').any(|c| c == rc) {
                    let lbl = sc_fmt_label(&r.get("shortcut").cloned().unwrap_or(Value::Null));
                    if !lbl.is_empty() {
                        return lbl;
                    }
                }
            }
            return String::new();
        }
        fallback.to_string()
    }

    fn open_feedback(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Share details (required)")
        });
        self.feedback_input = Some(input.clone());
        self.feedback_category = None;
        self.feedback_open = true;
        self.feedback_sending = false;
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// POST the dialog to upstream's public feedback endpoint
    /// (feedback.ts submitFeedback → trysynara.com/api/feedback).
    fn submit_feedback(&mut self, cx: &mut Context<Self>) {
        let details = self
            .feedback_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        if details.trim().is_empty() || self.feedback_sending {
            return;
        }
        self.feedback_sending = true;
        let cat = self
            .feedback_category
            .map(|i| Self::FEEDBACK_CATEGORIES[i]);
        let detail = self.detail.clone().unwrap_or(Value::Null);
        let thread_v = get(&detail, "thread");
        let provider = str_at(thread_v, "providerName").to_string();
        let messages_n = arr(thread_v, "messages").len();
        let activities_n = arr(thread_v, "activities").len();
        let session_status = session_phase(Some(&detail)).to_string();
        let runtime_mode = self.runtime_mode.to_string();
        let interaction_mode = self.interaction_mode.to_string();
        let lead = cat.map(|c| c.2).unwrap_or("I have some feedback");
        let body = json!({
            "category": cat.map(|c| c.0),
            "details": details.trim(),
            "summary": format!("{lead} in Synara 1.0.0."),
            "diagnostics": {
                "appVersion": "1.0.0",
                "submittedAt": now_iso(),
                "userAgent": "synara-shell/1.0.0",
                "platform": "macOS",
                "language": "en-US",
                "viewport": "0x0",
                "provider": if provider.is_empty() { Value::Null } else { Value::String(provider) },
                "model": Value::Null,
                "projectKind": Value::Null,
                "environmentMode": Value::Null,
                "runtimeMode": runtime_mode,
                "interactionMode": interaction_mode,
                "sessionStatus": session_status,
                "latestTurnState": Value::Null,
                "messageCount": messages_n,
                "activityCount": activities_n,
                "hasPendingApproval": false,
                "hasPendingUserInput": false,
                "hasThreadError": false,
            },
        });
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    ureq::post("https://www.trysynara.com/api/feedback")
                        .timeout(std::time::Duration::from_secs(20))
                        .set("x-synara-feedback", "1")
                        .send_json(body)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.feedback_sending = false;
                match res {
                    Ok(_) => {
                        this.feedback_open = false;
                        this.feedback_toast =
                            Some(("Feedback sent".to_string(), false));
                    }
                    Err(_) => {
                        this.feedback_toast =
                            Some(("Could not send feedback".to_string(), true));
                    }
                }
                cx.spawn(async move |this2: WeakEntity<Self>, cx2: &mut AsyncApp| {
                    cx2.background_executor()
                        .timer(std::time::Duration::from_secs(4))
                        .await;
                    let _ = this2.update(cx2, |this2: &mut Self, cx2| {
                        this2.feedback_toast = None;
                        cx2.notify();
                    });
                })
                .detach();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn insert_goal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let composer = self.composer.clone();
        self.composer.update(cx, |state, cx| {
            let current = state.value().to_string();
            if !current.trim_start().to_lowercase().starts_with("/goal") {
                let next = if current.trim().is_empty() {
                    "/goal ".to_string()
                } else {
                    format!("/goal {}", current)
                };
                state.set_value(next, window, cx);
            }
        });
        let focus = composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Attach-window rows: capture the CG window and upload it as an image
    /// attachment, mirroring upstream appSnap.captureWindow.
    fn capture_window(&mut self, window_id: String, title: String, cx: &mut Context<Self>) {
        let num = window_id.rsplit(':').next().unwrap_or("0").to_string();
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let tid = thread_id.unwrap_or_default();
            let uploaded: Option<Value> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return None;
                    };
                    let path = std::env::temp_dir().join(format!("synara-win-{num}.png"));
                    let ok = std::process::Command::new("screencapture")
                        .args(["-x", "-l", &num])
                        .arg(&path)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if !ok {
                        return None;
                    }
                    let Ok(bytes) = std::fs::read(&path) else {
                        return None;
                    };
                    let _ = std::fs::remove_file(&path);
                    let name = if title.is_empty() {
                        "window.png".to_string()
                    } else {
                        format!("{title}.png")
                    };
                    c.upload_attachment(&tid, "image", &name, "image/png", &bytes)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                if let Some(att) = uploaded {
                    this.pending_attachments.push(att);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC: GitHub inbox (PRs + issues of the project's repos).
    fn fetch_inbox(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = true;
                cx.notify();
            });
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let tasks = c.call("todo.list", json!({})).unwrap_or(Value::Null);
                    let recap = c.call("stats.getRecap", json!({})).unwrap_or(Value::Null);
                    let usage = c
                        .call("server.listProviderUsage", json!({}))
                        .unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    let provs = c
                        .call_first_chunk("server.subscribeProviderStatuses", json!({}))
                        .unwrap_or(Value::Null);
                    json!({"tasks": tasks, "recap": recap, "usage": usage, "account": acct, "providers": provs})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = false;
                this.inbox_tasks = arr(get(&bundle, "tasks"), "todos")
                    .to_vec()
                    .into_iter()
                    .chain(arr(get(&bundle, "tasks"), "items").to_vec())
                    .chain(if get(&bundle, "tasks").is_array() {
                        arr(&bundle, "tasks").to_vec()
                    } else {
                        Vec::new()
                    })
                    .collect();
                this.inbox_recap = Some(get(&bundle, "recap").clone());
                this.inbox_usage = arr(get(&bundle, "usage"), "snapshots")
                    .to_vec()
                    .into_iter()
                    .chain(arr(get(&bundle, "usage"), "rows").to_vec())
                    .chain(arr(get(&bundle, "usage"), "providers").to_vec())
                    .collect();
                let acct = get(&bundle, "account");
                let signed = get(acct, "signedIn")
                    .as_bool()
                    .or_else(|| get(acct, "authenticated").as_bool())
                    .or_else(|| Some(!str_at(acct, "accountId").is_empty()));
                this.inbox_signed_in = signed;
                // Upstream getNotifiableProviderUpdateStatuses: behind_latest,
                // latestVersion known, one-click update command present.
                this.inbox_updates = arr(get(&bundle, "providers"), "providers")
                    .to_vec()
                    .into_iter()
                    .filter(|p| {
                        let adv = get(p, "versionAdvisory");
                        str_at(adv, "status") == "behind_latest"
                            && !get(adv, "latestVersion").is_null()
                            && get(adv, "canUpdate").as_bool() == Some(true)
                            && !get(adv, "updateCommand").is_null()
                    })
                    .collect();
                cx.notify();
            });
        })
        .detach();
    }

    // ── Automations ─────────────────────────────────────────────────────

    /// automation.list → definitions + runs (upstream RailAutomationsPanel).
    fn fetch_automations(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    c.call("automation.list", json!({})).unwrap_or(Value::Null)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.autom_defs = arr(&bundle, "definitions").to_vec();
                this.autom_runs = arr(&bundle, "runs").to_vec();
                if let Some(sel) = &this.autom_sel {
                    if !this
                        .autom_defs
                        .iter()
                        .any(|d| str_at(d, "id") == sel.as_str())
                    {
                        this.autom_sel = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn autom_summary(schedule: &Value) -> String {
        let ty = str_at(schedule, "type");
        let time = str_at(schedule, "timeOfDay").to_string();
        match ty {
            "once" => format!("Once · {}", str_at(schedule, "runAt")),
            "interval" => {
                let s = schedule["everySeconds"].as_i64().unwrap_or(0);
                if s % 60 == 0 && s >= 60 {
                    format!("Every {} min", s / 60)
                } else {
                    format!("Every {}s", s)
                }
            }
            "daily" => format!("Daily at {time}"),
            "weekdays" => format!("Weekdays at {time}"),
            "weekly" => {
                let d = schedule["dayOfWeek"].as_i64().unwrap_or(0).clamp(0, 6) as usize;
                format!("Weekly {} at {}", AUTOM_DAYS[d], time)
            }
            "cron" => str_at(schedule, "expression").to_string(),
            "project-event" => "On project events".to_string(),
            other => other.to_string(),
        }
    }

    fn open_autom_dialog(
        &mut self,
        edit: Option<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut mk = |cx: &mut Context<Self>, ph: &'static str, val: &str| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(ph));
            if !val.is_empty() {
                let v = val.to_string();
                input.update(cx, |s, cx| s.set_value(v, window, cx));
            }
            input
        };
        let (mut edit_id, mut kind, mut day, mut enabled, mut time_v, mut extra_v) =
            (None, 2usize, 1usize, true, String::new(), String::new());
        let (mut name_v, mut prompt_v, mut proj_id) =
            (String::new(), String::new(), String::new());
        if let Some(def) = &edit {
            edit_id = Some(str_at(def, "id").to_string());
            name_v = str_at(def, "name").to_string();
            prompt_v = str_at(def, "prompt").to_string();
            enabled = get(def, "enabled").as_bool().unwrap_or(true);
            proj_id = str_at(def, "projectId").to_string();
            let sched = get(def, "schedule");
            match str_at(sched, "type") {
                "once" => {
                    kind = 0;
                    extra_v = str_at(sched, "runAt").to_string();
                }
                "interval" => {
                    kind = 1;
                    extra_v = (sched["everySeconds"].as_i64().unwrap_or(0) / 60).to_string();
                }
                "daily" => {
                    kind = 2;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "weekdays" => {
                    kind = 3;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "weekly" => {
                    kind = 4;
                    day = sched["dayOfWeek"].as_i64().unwrap_or(1).clamp(0, 6) as usize;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "cron" => {
                    kind = 5;
                    extra_v = str_at(sched, "expression").to_string();
                }
                _ => {}
            }
        }
        let projects = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects");
        let mut proj = 0usize;
        if !proj_id.is_empty() {
            if let Some(i) = projects
                .iter()
                .position(|p| str_at(p, "id") == proj_id.as_str())
            {
                proj = i;
            }
        }
        let name = mk(cx, "Automation name", &name_v);
        let prompt = mk(cx, "What should this automation do?", &prompt_v);
        let time = mk(cx, "HH:MM", &time_v);
        let extra_ph = match kind {
            0 => "Run at (YYYY-MM-DD HH:MM)",
            1 => "Every N minutes",
            5 => "Cron expression",
            _ => "Value",
        };
        let extra = mk(cx, extra_ph, &extra_v);
        self.autom_dialog = Some(AutomForm {
            edit_id,
            name,
            prompt,
            time,
            extra,
            kind,
            day,
            proj,
            enabled,
        });
        if let Some(f) = &self.autom_dialog {
            let focus = f.name.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn close_autom_dialog(&mut self, cx: &mut Context<Self>) {
        self.autom_dialog = None;
        cx.notify();
    }

    fn autom_run_now(&mut self, id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call("automation.runNow", json!({"automationId": id})) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_automations(cx);
            });
        })
        .detach();
    }

    fn autom_delete(&mut self, id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call("automation.delete", json!({"automationId": id})) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.autom_sel = None;
                this.fetch_automations(cx);
            });
        })
        .detach();
    }

    fn submit_autom_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.autom_dialog.take() else {
            return;
        };
        let name = form.name.read(cx).value().trim().to_string();
        let prompt = form.prompt.read(cx).value().trim().to_string();
        let time = form.time.read(cx).value().trim().to_string();
        let extra = form.extra.read(cx).value().trim().to_string();
        let projects = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects");
        let project_id = projects
            .get(form.proj)
            .map(|p| str_at(p, "id").to_string())
            .unwrap_or_default();
        if name.is_empty() || prompt.is_empty() || project_id.is_empty() {
            self.autom_dialog = Some(form);
            cx.notify();
            return;
        }
        let time_of_day = if time.is_empty() {
            "09:00".to_string()
        } else {
            time
        };
        let schedule = match form.kind {
            0 => {
                let mut at = extra.replace(' ', "T");
                if at.len() == 16 {
                    at.push_str(":00Z");
                } else if !at.ends_with('Z') && !at.contains('+') {
                    at.push('Z');
                }
                json!({"type":"once","runAt":at})
            }
            1 => json!({
                "type":"interval",
                "everySeconds": extra.parse::<i64>().unwrap_or(60).max(1) * 60,
            }),
            2 => json!({"type":"daily","timeOfDay":time_of_day}),
            3 => json!({"type":"weekdays","timeOfDay":time_of_day}),
            4 => json!({"type":"weekly","dayOfWeek":form.day,"timeOfDay":time_of_day}),
            _ => json!({"type":"cron","expression":extra,"timezone":"UTC"}),
        };
        let payload = json!({
            "projectId": project_id,
            "name": name,
            "prompt": prompt,
            "schedule": schedule,
            "enabled": form.enabled,
        });
        let (tag, payload) = if let Some(id) = &form.edit_id {
            let mut p = payload;
            p["id"] = json!(id);
            ("automation.update", p)
        } else {
            ("automation.create", payload)
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call(tag, payload) })
                .detach();
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_automations(cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// Toggle a task's completed state from the Inbox tile (upstream
    /// TaskListItem checkbox → todo.update {completed}).
    fn toggle_todo(&mut self, id: String, done: bool, cx: &mut Context<Self>) {
        // Optimistic flip so the row repaints before the round-trip lands.
        for task in self.inbox_tasks.iter_mut() {
            if str_at(task, "id") == id {
                if let Some(m) = task.as_object_mut() {
                    m.insert(
                        "completedAt".into(),
                        if done {
                            json!(chrono::Utc::now().to_rfc3339())
                        } else {
                            Value::Null
                        },
                    );
                }
            }
        }
        let cell = self.client.clone();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let id = id.clone();
            let _ = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("todo.update", json!({"id": id, "completed": done})))
                })
                .await;
        })
        .detach();
        cx.notify();
    }

    /// Blocking RPC bundle for the settings panes: provider statuses,
    /// usage quotas, skills, connectors, integrations, worktrees,
    /// profile stats, computer backend status, archived thread count.
    fn fetch_settings(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.settings_loading = true;
                cx.notify();
            });
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let cfg = c.call("server.getConfig", json!({})).unwrap_or(Value::Null);
                    let app = c.call("server.getSettings", json!({})).unwrap_or(Value::Null);
                    let usage = c.call("server.listProviderUsage", json!({})).unwrap_or(Value::Null);
                    let skills = c.call("provider.listSkills", json!({})).unwrap_or(Value::Null);
                    let conns = c.call("server.listMcpConnectors", json!({})).unwrap_or(Value::Null);
                    let props = c.call("server.listMcpConnectorProposals", json!({})).unwrap_or(Value::Null);
                    let exts = c.call("server.listExternalMcpIntegrations", json!({})).unwrap_or(Value::Null);
                    let wts = c.call("server.listWorktrees", json!({})).unwrap_or(Value::Null);
                    let prof = c.call("stats.getProfileStats", json!({})).unwrap_or(Value::Null);
                    let toks = c.call("stats.getProfileTokenStats", json!({})).unwrap_or(Value::Null);
                    // Upstream default "30d" range, previous period included
                    // (usageAnalyticsWindow): from = local midnight today-59,
                    // to = local midnight tomorrow, scope "synara".
                    let today = chrono::Local::now().date_naive();
                    // Upstream serializes with Date.toISOString(): UTC "Z"
                    // (usageAnalyticsWindow), not a local-offset RFC3339.
                    let ana_from = today
                        .checked_sub_days(chrono::Days::new(59))
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                        .map(|d| {
                            d.and_local_timezone(chrono::Local)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                                .to_string()
                        });
                    let ana_to = today
                        .checked_add_days(chrono::Days::new(1))
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                        .map(|d| {
                            d.and_local_timezone(chrono::Local)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                                .to_string()
                        });
                    let ana = match (ana_from, ana_to) {
                        (Some(f), Some(t)) => c
                            .call(
                                "stats.getUsageAnalytics",
                                json!({"from": f, "to": t, "scope": "synara"}),
                            )
                            .unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                    let comp = c.call("computer.getStatus", json!({})).unwrap_or(Value::Null);
                    let snap = c.call("orchestration.getShellSnapshot", json!({})).unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    json!({"config":cfg,"settings":app,"usage":usage,"skills":skills,
                           "connectors":conns,"proposals":props,"external":exts,
                           "worktrees":wts,"profile":prof,"tokens":toks,"analytics":ana,
                           "computer":comp,"snapshot":snap,"account":acct})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(rules) = bundle
                    .get("config")
                    .and_then(|c| c.get("keybindings"))
                    .and_then(|k| k.as_array())
                    .cloned()
                {
                    this.kb_rules = rules;
                }
                this.settings_loading = false;
                this.settings_data = bundle;
                // Appearance (upstream appSettings, server-side mirror):
                // mode / uiDensity / chatWidth / showAsciiBackground.
                let app = get(&this.settings_data, "settings");
                let ap = get(app, "appearance");
                if ap.is_object() {
                    this.appearance_mode = match str_at(ap, "mode") {
                        "light" => 1,
                        "dark" => 2,
                        _ => 0,
                    };
                    this.ui_density = match str_at(ap, "uiDensity") {
                        "compact" => 0,
                        "spacious" => 2,
                        _ => 1,
                    };
                    this.chat_width_mode = match str_at(ap, "chatWidth") {
                        "wide" => 1,
                        "full" => 2,
                        _ => 0,
                    };
                    this.show_ascii_bg = ap
                        .get("showAsciiBackground")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                }
                this.keep_awake_mode = match str_at(app, "keepAwakeMode") {
                    "on" => 0,
                    "agent" => 1,
                    _ => 2,
                };
                // Sidebar UI state (upstream localStorage mirrors): dismissed
                // status keys + last-visited stamps drive pill suppression.
                let su = get(get(&this.settings_data, "settings"), "sidebarUi").clone();
                this.apply_appearance_mode(None, cx);
                if let Some(m) = get(&su, "dismissedStatusKeyByThreadId").as_object() {
                    this.dismissed_status = m
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                }
                if let Some(m) = get(&su, "lastVisitedAtByThreadId").as_object() {
                    this.visited_threads = m
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                }
                if let Some(m) = get(&su, "coachmarks").as_object() {
                    this.coachmarks_seen = m.keys().cloned().collect();
                }
                if let Some(m) = get(&su, "featureFlags").as_object() {
                    const FF_KEYS: [&str; 3] = [
                        "persist-action-failed-debug-toasts",
                        "pin-git-progress-toast-preview",
                        "show-debug-task-banner",
                    ];
                    for (i, k) in FF_KEYS.iter().enumerate() {
                        this.feature_flags[i] =
                            m.get(*k).and_then(|v| v.as_bool()).unwrap_or(false);
                    }
                }
                this.show_next_coachmark(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn density_scale(&self) -> f32 {
        [0.85_f32, 1.0, 1.15][self.ui_density as usize]
    }

    /// Upstream CHAT_MAX_WIDTH_BY_MODE: standard 46rem, wide 72rem, full 100%.
    fn chat_max_w(&self) -> Pixels {
        match self.chat_width_mode {
            0 => px(726.),
            1 => px(1152.),
            _ => px(100_000.),
        }
    }

    /// Persists the appearance map under settings.appearance, like
    /// upstream's appSettings write-through (server.updateSettings patch).
    fn persist_appearance(&mut self, cx: &mut Context<Self>) {
        let mode = ["system", "light", "dark"][self.appearance_mode as usize];
        let dens = ["compact", "comfortable", "spacious"][self.ui_density as usize];
        let width = ["standard", "wide", "full"][self.chat_width_mode as usize];
        let appearance = json!({
            "mode": mode,
            "uiDensity": dens,
            "chatWidth": width,
            "showAsciiBackground": self.show_ascii_bg,
        });
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("appearance".to_string(), appearance.clone());
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"appearance": appearance});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Upstream keep-awake segmented control: writes settings.keepAwakeMode
    /// ("on" | "agent" | "off"); the server turns that into caffeinate state.
    fn set_keep_awake(&mut self, i: usize, cx: &mut Context<Self>) {
        self.keep_awake_mode = i;
        let mode = ["on", "agent", "off"][i.min(2)];
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("keepAwakeMode".to_string(), json!(mode));
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"keepAwakeMode": mode});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Upstream keepAwakeStatusLabel: "<Mode> · <Active|Idle>".
    fn keep_awake_status(&self) -> String {
        let active = self.keep_awake_mode == 0
            || (self.keep_awake_mode == 1
                && arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .any(|t| thread_running_for_activity(t)));
        format!(
            "{} \u{00b7} {}",
            ["On", "Agent", "Off"][self.keep_awake_mode.min(2)],
            if active { "Active" } else { "Idle" }
        )
    }

    /// Upstream OneTimeCoachmark: a single accent hint at a time, seen-keyed,
    /// marked seen the moment it opens, auto-hidden after 8s.
    fn show_next_coachmark(&mut self, cx: &mut Context<Self>) {
        if self.coachmark.is_some() {
            return;
        }
        let next = if !self.coachmarks_seen.contains("activity") {
            "activity"
        } else if !self.coachmarks_seen.contains("tasks") {
            "tasks"
        } else {
            return;
        };
        self.coachmark = Some(next);
        self.coachmarks_seen.insert(next.to_string());
        self.persist_sidebar_ui(cx);
        cx.notify();
        let weak = cx.weak_entity();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(8))
                .await;
            let _ = weak.update(cx, |this: &mut Self, cx| {
                if this.coachmark == Some(next) {
                    this.coachmark = None;
                    this.show_next_coachmark(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Clicking the wrapped control dismisses its hint (upstream marks it
    /// seen at open, so no extra write is needed here).
    fn dismiss_coachmark(&mut self, key: &'static str, cx: &mut Context<Self>) {
        if self.coachmark == Some(key) {
            self.coachmark = None;
            self.show_next_coachmark(cx);
            cx.notify();
        }
    }

    /// Upstream DebugFeatureFlagsMenu toggle — flips flag `i` and persists
    /// the whole map under settings.sidebarUi.featureFlags.
    fn toggle_feature_flag(&mut self, i: usize, cx: &mut Context<Self>) {
        self.feature_flags[i] = !self.feature_flags[i];
        // "Pin git progress toast" keeps a visible progress toast while on.
        if i == 1 {
            self.feedback_toast = if self.feature_flags[1] {
                Some(("Pushing changes\u{2026}".to_string(), false))
            } else {
                None
            };
        }
        self.persist_sidebar_ui(cx);
    }

    /// Upstream "Trigger action failed toasts" — fires stacked Git-failure
    /// error toasts for local UI testing.
    fn trigger_debug_toasts(&mut self, cx: &mut Context<Self>) {
        self.feedback_toast = Some(("Git action failed.".to_string(), true));
        cx.notify();
    }

    /// Persists dismissal/visited maps under settings.sidebarUi, like
    /// upstream's localStorage write-through on dismiss / mark-visited.
    fn persist_sidebar_ui(&mut self, cx: &mut Context<Self>) {
        let sidebar_ui = json!({
            "dismissedStatusKeyByThreadId": self.dismissed_status,
            "lastVisitedAtByThreadId": self.visited_threads,
            "coachmarks": self
                .coachmarks_seen
                .iter()
                .map(|k| (k.clone(), json!("seen")))
                .collect::<serde_json::Map<String, Value>>(),
            "featureFlags": {
                "persist-action-failed-debug-toasts": self.feature_flags[0],
                "pin-git-progress-toast-preview": self.feature_flags[1],
                "show-debug-task-banner": self.feature_flags[2],
            },
        });
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("sidebarUi".to_string(), sidebar_ui.clone());
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"sidebarUi": sidebar_ui});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    fn apply_appearance_mode(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        let mode = match self.appearance_mode {
            1 => gpui_kit::component::ThemeMode::Light,
            2 => gpui_kit::component::ThemeMode::Dark,
            _ => gpui_kit::component::ThemeMode::from(cx.window_appearance()),
        };
        gpui_kit::component::Theme::change(mode, w, cx);
    }

    fn set_appearance_mode(&mut self, mode: u8, w: &mut Window, cx: &mut Context<Self>) {
        self.appearance_mode = mode;
        self.apply_appearance_mode(Some(w), cx);
        self.persist_appearance(cx);
    }

    fn set_ui_density(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.ui_density = ix as u8;
        self.persist_appearance(cx);
    }

    fn set_chat_width(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.chat_width_mode = ix as u8;
        self.persist_appearance(cx);
    }

    fn toggle_ascii_bg(&mut self, cx: &mut Context<Self>) {
        self.show_ascii_bg = !self.show_ascii_bg;
        self.persist_appearance(cx);
    }

    /// Segmented control with real selection (the static `seg` renders the
    /// same visual; this one drives `act(ix)` on click).
    fn seg_pick(
        cx: &mut Context<Self>,
        idp: &'static str,
        opts: &[&'static str],
        active: usize,
        theme: &gpui_kit::component::Theme,
        act: fn(&mut Self, usize, &mut Context<Self>),
    ) -> gpui::AnyElement {
        let mut s = h_flex().items_center().gap(px(4.));
        for (i, o) in opts.iter().enumerate() {
            let is_active = i == active;
            let mut c = div()
                .id(ElementId::Name(format!("{idp}-{i}").into()))
                .h(px(32.))
                .px(px(11.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer();
            if is_active {
                c = c.bg(theme.secondary);
            }
            let tc = if is_active { theme.foreground } else { theme.muted_foreground };
            s = s.child(
                c.child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(tc).child(*o))
                    .on_click(cx.listener(move |this, _ev, _w, cx| act(this, i, cx))),
            );
        }
        s.into_any_element()
    }

    /// Upstream rail "…" checkbox: toggles `key` inside
    /// settings.railShortcuts and persists via server.updateSettings.
    fn toggle_rail_shortcut(&mut self, key: &str, cx: &mut Context<Self>) {
        let mut next: Vec<String> = arr(
            get(&self.settings_data, "settings"),
            "railShortcuts",
        )
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
        if let Some(pos) = next.iter().position(|k| k == key) {
            next.remove(pos);
        } else {
            next.push(key.to_string());
        }
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert(
                "railShortcuts".to_string(),
                json!(next.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
            );
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"railShortcuts": next});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Blocking RPC: list running dev servers.
    fn fetch_dev_servers(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("projects.listDevServers", json!({})))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.dev_servers = v.map(|v| arr(&v, "servers").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    /// Start a dev server for the first project with the typed command.
    fn dev_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let command = self.dev_cmd.read(cx).value().to_string();
        let command = command.trim().to_string();
        if command.is_empty() {
            return;
        }
        self.dev_cmd.update(cx, |s, cx| s.clean(window, cx));
        let project_id = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("default")
            .to_string();
        let Some(cwd) = self.files_root() else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.runDevServer",
                            json!({"projectId": project_id, "command": command, "cwd": cwd}),
                        )
                    })
                })
                .await;
            if v.is_none() {
                eprintln!("native: runDevServer failed");
            }
            let _ = this.update(cx, |this: &mut Self, cx| this.fetch_dev_servers(cx));
        })
        .detach();
    }

    /// Stop a dev server by project id.
    fn dev_stop(&mut self, project_id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call("projects.stopDevServer", json!({"projectId": project_id}))
                    })
                })
                .await;
            if v.is_none() {
                eprintln!("native: stopDevServer failed");
            }
            let _ = this.update(cx, |this: &mut Self, cx| this.fetch_dev_servers(cx));
        })
        .detach();
    }

    /// Debounced content search via orchestration.searchThreads.
    fn on_search_change(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        self.search_query = query.clone();
        self.search_hits = Vec::new();
        cx.notify();
        let trimmed = query.trim().to_string();
        if trimmed.chars().count() < 2 {
            return;
        }
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            let still = this
                .update(cx, |this: &mut Self, _| this.search_query.trim() == trimmed)
                .unwrap_or(false);
            if !still {
                return;
            }
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "orchestration.searchThreads",
                            json!({"query": trimmed, "limit": 20}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.search_hits = v.map(|v| arr(&v, "matches").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    /// Workspace root of the selected thread's project, if resolvable.
    fn thread_project_root(&self) -> Option<String> {
        let tid = self.selected_thread.as_deref()?;
        let snap = self.snapshot.as_ref()?;
        let pid = arr(snap, "threads")
            .iter()
            .find(|t| str_at(t, "id") == tid)
            .map(|t| str_at(t, "projectId").to_string())?;
        let root = arr(snap, "projects")
            .iter()
            .find(|p| str_at(p, "id") == pid)
            .map(|p| {
                let r = str_at(p, "workspaceRoot");
                if r.is_empty() { str_at(p, "workspace_root") } else { r }
            })
            .filter(|r| !r.is_empty());
        root.map(|r| r.to_string())
    }

    /// Fetch git.status of the selected thread's project root (fallback: first project).
    fn fetch_git_status(&self, cx: &mut Context<Self>) {
        let Some(root) = self
            .thread_project_root()
            .or_else(|| self.files_root())
        else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("git.status", json!({"cwd": root})))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.git_status = v;
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC: list one directory level (dirs + files).
    /// Draft typing hook: keep the "@" mention browser pointed at the
    /// directory the query resolves to (upstream useComposerDiscovery).
    fn on_composer_change(&mut self, cx: &mut Context<Self>) {
        let draft = self.composer.read(cx).value().to_string();
        if let Some(dir) = mention_browse_dir(&draft) {
            if dir != self.mention_dir {
                self.load_mention_dir(dir, cx);
            }
        }
        cx.notify();
    }

    /// `projects.listDirectories` into the mention browser state (kept
    /// separate from the file-browser pane's `files`).
    fn load_mention_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.mention_seq += 1;
        let seq = self.mention_seq;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let d = dir.clone();
            let entries = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.listDirectories",
                            json!({"cwd": d, "depth": 1, "includeFiles": true}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if seq != this.mention_seq {
                    return;
                }
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.mention_entries = es;
                    this.mention_dir = dir;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Files-pane fuzzy search: 120ms debounce into `projects.searchEntries`
    /// (upstream useWorkspaceFileSearch), limit 80, files only.
    fn on_file_search_change(&mut self, cx: &mut Context<Self>) {
        self.file_search_seq += 1;
        let seq = self.file_search_seq;
        let query = self.file_search.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.file_search_results.clear();
            cx.notify();
            return;
        }
        let dir = self.files_dir.clone();
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(120))
                .await;
            if this
                .update(cx, |this: &mut Self, _cx| seq != this.file_search_seq)
                .unwrap_or(true)
            {
                return;
            }
            let q = query.clone();
            let d = dir.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.searchEntries",
                            json!({"cwd": d, "query": q, "limit": 80, "kind": "file"}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if seq != this.file_search_seq {
                    return;
                }
                this.file_search_results = res
                    .map(|v| arr(&v, "entries").to_vec())
                    .unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    fn load_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.files_loading = true;
        self.open_file = None;
        self.tree_entries.clear();
        self.expanded_dirs.clear();
        self.tree_loading.clear();
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let d = dir.clone();
            let entries = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.listDirectories",
                            json!({"cwd": d, "depth": 1, "includeFiles": true}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.files_loading = false;
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.files = es;
                    this.files_dir = dir;
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Open a file entry for reading, or descend into a directory entry.
    fn open_entry(&mut self, entry: Value, cx: &mut Context<Self>) {
        let kind = str_at(&entry, "kind").to_string();
        let path = str_at(&entry, "path").to_string();
        if kind == "directory" {
            self.load_dir(path, cx);
            return;
        }
        if kind != "file" {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let p = path.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    let dir = p
                        .rsplit_once('/')
                        .map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| p.clone());
                    let name = p.rsplit('/').next().unwrap_or(&p).to_string();
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.readFile",
                            json!({"cwd": dir, "relativePath": name}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = v {
                    let text = str_at(&v, "contents").to_string();
                    this.open_file_meta = Some((
                        str_at(&v, "version").to_string(),
                        str_at(&v, "encoding").to_string(),
                        str_at(&v, "lineEnding").to_string(),
                    ));
                    this.open_file = Some((path, text));
                    this.watch_open_file(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Expand/collapse a directory row in the explorer tree (upstream
    /// onToggleDirectory). The first expand lazily fetches the listing;
    /// later toggles reuse the cached `tree_entries`.
    fn toggle_tree_dir(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.expanded_dirs.remove(&path) {
            self.expanded_dirs.insert(path.clone());
            if !self.tree_entries.contains_key(&path) {
                self.load_tree_dir(path, cx);
            }
        }
        cx.notify();
    }

    /// Lazy-load one directory level for the explorer tree (upstream
    /// WorkspaceDirectory → projects.listDirectories).
    fn load_tree_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        if !self.tree_loading.insert(dir.clone()) {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let d = dir.clone();
            let entries = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.listDirectories",
                            json!({"cwd": d, "depth": 1, "includeFiles": true}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.tree_loading.remove(&dir);
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.tree_entries.insert(dir, es);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Rows of one level of the explorer tree (upstream WorkspaceDirectory +
    /// ExplorerRow): h-7 rows indented 8+12·depth px, disclosure chevron on
    /// directories, selected block on the open file, lazy expanded children.
    fn file_tree_rows(
        &self,
        entries: &[Value],
        depth: usize,
        theme: &Theme,
        subtle: Hsla,
        out: &mut Vec<gpui::AnyElement>,
        cx: &mut Context<Self>,
    ) {
        for entry in entries {
            if !explorer_entry_visible(entry) {
                continue;
            }
            let name = str_at(entry, "name").to_string();
            let path = str_at(entry, "path").to_string();
            let is_dir = str_at(entry, "kind") == "directory";
            let expanded = is_dir && self.expanded_dirs.contains(&path);
            let selected = !is_dir
                && self
                    .open_file
                    .as_ref()
                    .map(|(p, _)| p == &path)
                    .unwrap_or(false);
            let indent = px(8. + depth as f32 * 12.);
            let entry_c = entry.clone();
            let path_c = path.clone();
            let mut row = h_flex()
                .id(ElementId::Name(format!("file-{path}").into()))
                .cursor_pointer()
                .h(px(28.))
                .items_center()
                .gap_1p5()
                .pl(indent)
                .pr_2()
                .rounded_md()
                .child(
                    svg_icon(if is_dir {
                        if expanded {
                            icons::CHEVRON_DOWN
                        } else {
                            icons::CHEVRON_RIGHT
                        }
                    } else {
                        icons::FILE_TEXT
                    })
                    .size(px(14.))
                    .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(name),
                );
            if selected {
                row = row.bg(theme.secondary);
            } else {
                row = row.hover(move |d| d.bg(subtle));
            }
            row = row.on_click(cx.listener(move |this, _ev, _w, cx| {
                if is_dir {
                    this.toggle_tree_dir(path_c.clone(), cx);
                } else {
                    this.open_entry(entry_c.clone(), cx);
                }
            }));
            let path_rc = path.clone();
            row = row.on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    this.ctx_menu = None;
                    this.file_ctx_menu =
                        Some((path_rc.clone(), ev.position.x.into(), ev.position.y.into()));
                    cx.stop_propagation();
                    cx.notify();
                }),
            );
            // Upstream drag-to-chat: dragging a file onto the composer
            // references it.
            let ghost_name = str_at(entry, "name").to_string();
            let path_drag = path.clone();
            row = row.on_drag(FileDrag(path_drag), move |_d, _pos, _w, cx| {
                cx.new(|_| DragGhost(ghost_name.clone()))
            });
            out.push(row.into_any_element());
            if expanded {
                if let Some(children) = self.tree_entries.get(&path) {
                    let kids = children.clone();
                    self.file_tree_rows(&kids, depth + 1, theme, subtle, out, cx);
                } else if self.tree_loading.contains(&path) {
                    out.push(
                        div()
                            .pl(px(8. + (depth + 1) as f32 * 12.))
                            .py(px(4.))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("loading\u{2026}")
                            .into_any_element(),
                    );
                }
            }
        }
    }

    /// Subscribe to change events for the currently open file; each event
    /// triggers a re-read so the viewer stays live.
    fn watch_open_file(&mut self, cx: &mut Context<Self>) {
        self.stop_file_watch();
        let Some((path, _)) = self.open_file.clone() else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let (tx, rx) = async_channel::unbounded::<()>();
            let cell2 = cell.clone();
            let p = path.clone();
            let got = cx
                .background_executor()
                .spawn(async move {
                    let dir = p
                        .rsplit_once('/')
                        .map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| p.clone());
                    let name = p.rsplit('/').next().unwrap_or(&p).to_string();
                    ensure_client(&cell2).and_then(|c| {
                        c.request(
                            "projects.subscribeFileChange",
                            json!({"cwd": dir, "relativePath": name}),
                        )
                    })
                })
                .await;
            let Some((rid, frames)) = got else {
                return;
            };
            let _ = this.update(cx, |this: &mut Self, _cx| {
                this.file_stream_id = Some(rid);
            });
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                })
                .detach();
            while rx.recv().await.is_ok() {
                let cell3 = cell.clone();
                let p2 = path.clone();
                let v = cx
                    .background_executor()
                    .spawn(async move {
                        let dir = p2
                            .rsplit_once('/')
                            .map(|(d, _)| d.to_string())
                            .unwrap_or_else(|| p2.clone());
                        let name = p2.rsplit('/').next().unwrap_or(&p2).to_string();
                        ensure_client(&cell3).and_then(|c| {
                            c.call(
                                "projects.readFile",
                                json!({"cwd": dir, "relativePath": name}),
                            )
                        })
                    })
                    .await;
                let still_open = this
                    .update(cx, |this: &mut Self, cx| {
                        if let Some(v) = v {
                            if this
                                .open_file
                                .as_ref()
                                .is_some_and(|(p, _)| *p == path)
                            {
                                let text = str_at(&v, "contents").to_string();
                                this.open_file_meta = Some((
                                    str_at(&v, "version").to_string(),
                                    str_at(&v, "encoding").to_string(),
                                    str_at(&v, "lineEnding").to_string(),
                                ));
                                this.open_file = Some((path.clone(), text));
                            }
                        }
                        cx.notify();
                    })
                    .is_ok();
                if !still_open {
                    break;
                }
            }
        })
        .detach();
    }

    fn stop_file_watch(&mut self) {
        if let Some(old) = self.file_stream_id.take() {
            if let Ok(g) = self.client.lock() {
                if let Some(c) = g.as_ref() {
                    c.interrupt(&old);
                }
            }
        }
    }

    fn close_file(&mut self, cx: &mut Context<Self>) {
        self.file_dirty = false;
        self.open_in_menu = false;
        self.stop_file_watch();
        self.open_file = None;
        self.open_file_meta = None;
        self.file_editor = None;
        self.editor_synced = None;
        self.editing_file = false;
        cx.notify();
    }

    /// Map a file path to an editor language id for highlighting.
    fn editor_language(path: &str) -> &'static str {
        let ext = path.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "json" => "json",
            "md" => "markdown",
            "py" => "python",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "html" => "html",
            "css" => "css",
            "sh" | "bash" | "zsh" => "shell",
            _ => "",
        }
    }

    /// Save the edited file back via projects.writeFile, guarded by the
    /// version reported by the last read.
    fn save_file(&mut self, cx: &mut Context<Self>) {
        let Some((path, _)) = self.open_file.clone() else {
            return;
        };
        let Some(editor) = self.file_editor.clone() else {
            return;
        };
        let contents = editor.read(cx).value().to_string();
        let (version, encoding, line_ending) =
            self.open_file_meta.clone().unwrap_or_default();
        let dir = path
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_else(|| path.clone());
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let p = path.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    let mut payload = json!({
                        "cwd": dir,
                        "relativePath": name,
                        "contents": contents,
                    });
                    if !version.is_empty() {
                        payload["expectedVersion"] = json!(version);
                        if !encoding.is_empty() {
                            payload["encoding"] = json!(encoding);
                        }
                        if !line_ending.is_empty() {
                            payload["lineEnding"] = json!(line_ending);
                        }
                    }
                    ensure_client(&cell)
                        .and_then(|c| c.call("projects.writeFile", payload))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = v {
                    if let Some(ed) = &this.file_editor {
                        let text = ed.read(cx).value().to_string();
                        this.open_file = Some((p, text));
                    }
                    let meta = this.open_file_meta.get_or_insert_default();
                    if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                        meta.0 = ver.to_string();
                        this.editor_synced = Some(ver.to_string());
                    }
                    this.editing_file = false;
                    this.file_dirty = false;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch a simple thread command (interrupt, archive, delete, ...).
    fn dispatch_thread_cmd(&mut self, cmd_type: &'static str, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": cmd_type,
                "commandId": uuid(),
                "threadId": tid,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: {cmd_type} dispatch failed");
            }
            let cell = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getShellSnapshot", json!({})))
                })
                .await;
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch thread.approval.respond for a pending interaction.
    fn respond_approval(&mut self, request_id: String, decision: &str, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let decision = decision.to_string();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": "thread.approval.respond",
                "commandId": uuid(),
                "threadId": tid,
                "requestId": request_id,
                "decision": decision,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: approval.respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Answer an async user-input card. Upstream (useAsyncUserInputResponse)
    /// steers a turn whose message is formatAsyncUserInputResponse and whose
    /// asyncUserInputResponse binds the answers to the question message.
    fn respond_user_input(
        &mut self,
        msg_id: String,
        questions: Vec<String>,
        answers: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let text = questions
                .iter()
                .zip(answers.iter())
                .map(|(q, a)| format!("{q}\n{a}"))
                .collect::<Vec<_>>()
                .join("\n\n");
            let cell = client.clone();
            let t2 = tid.clone();
            let cmd = json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": t2,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "asyncUserInputResponse": {
                    "messageId": msg_id,
                    "answers": answers,
                },
                "dispatchMode": "steer",
                "runtimeMode": mode,
                "interactionMode": imode,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: user-input respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Upstream stop affordance: dispatch thread.turn.interrupt and hold the
    /// queued follow-ups (upstream pauses the waiting queue on a user Stop).
    fn stop(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        if !self.queued_turns.is_empty() {
            self.queue_paused = true;
        }
        let client = self.client.clone();
        let cmd = json!({
            "type": "thread.turn.interrupt",
            "requestedBy": "user",
            "commandId": uuid(),
            "threadId": tid,
            "createdAt": now_iso(),
        });
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            if let Some(client) = client {
                let c = client.clone();
                let _ = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                    .await;
                let c = client.clone();
                let p = json!({"threadId": tid});
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.detail = d;
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// Composer: send the draft as a turn on the selected thread, or create a
    /// thread in the first project then start the turn.

    /// Transcript surface shared by the primary pane and the split pane.
    /// `detail` is the thread detail snapshot (may be null); `has_thread`
    /// and `show_landing` mirror the caller's state for the empty row.
    fn transcript_body(
        &self,
        detail: &Value,
        has_thread: bool,
        show_landing: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let subtle = if dark_ui { rgba(0xffffff14) } else { rgba(0x0000000d) };
        // Upstream fills are neutral, not the navy of gpui's default
        // list_active: selected rows = white 4%, user bubble = #181818.
        let sel_fill: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        };
        let user_bubble: gpui::Hsla = if dark_ui {
            rgb(0x181818).into()
        } else {
            theme.list_active
        };
        let mut col = v_flex().gap_2();
        let thread_v = get(detail, "thread");
        let messages = arr(thread_v, "messages");
        let find_q = self.find_query.to_lowercase();
        let chat_mw = self.chat_max_w();
        if messages.is_empty() && has_thread && !show_landing {
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Select a thread or type below"),
                    ),
                ),
            );
        }
        for (i, m) in messages.iter().enumerate() {
            let role = str_at(m, "role");
            let mut text = str_at(m, "text").to_string();
            if text.trim().is_empty() {
                text = get(m, "textSegments")
                    .as_array()
                    .map(|segs| {
                        segs.iter()
                            .filter_map(|s| s.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
            }
            // Async user-input card (upstream AsyncUserInputCard): question
            // rows with option picks, submitted as a steered turn.
            let aui = get(m, "asyncUserInput");
            let questions = arr(aui, "questions");
            if !questions.is_empty() {
                let msg_id = str_at(m, "id").to_string();
                let done: Vec<String> = arr(get(aui, "response"), "answers")
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                let answered = !done.is_empty();
                let n = questions.len();

                let mut card = v_flex()
                    .w_full()
                    .gap_3()
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(12.))
                    .px(px(14.))
                    .py(px(14.));
                // Header pill: "? N question(s)" (+ "✓ Answered").
                let mut head = h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        svg_icon(icons::HELP)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{n} {}",
                                if n == 1 { "question" } else { "questions" }
                            )),
                    );
                if answered {
                    head = head
                        .child(
                            svg_icon(icons::CHECK)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Answered"),
                        );
                }
                card = card.child(head);

                if answered {
                    for (qi, q) in questions.iter().enumerate() {
                        let mut qa = v_flex().gap_1().child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(str_at(q, "title").to_string()),
                        );
                        if let Some(a) = done.get(qi) {
                            qa = qa.child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(a.clone()),
                            );
                        }
                        card = card.child(qa);
                    }
                } else {
                    // Effective pick per question: draft override, else the
                    // first option (upstream preselects options[0]).
                    let mut all_answerable = true;
                    for (qi, q) in questions.iter().enumerate() {
                        let title = str_at(q, "title").to_string();
                        let options: Vec<String> = arr(q, "options")
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect();
                        let pick = self
                            .user_input_draft
                            .iter()
                            .find(|(id, i, _)| id == &msg_id && *i == qi)
                            .map(|(_, _, l)| l.clone())
                            .or_else(|| options.first().cloned());
                        let mut qv = v_flex().gap_2().child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        );
                        if options.is_empty() {
                            all_answerable = false;
                            qv = qv.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("Type your answer in the composer below"),
                            );
                        } else {
                            let mut orow = h_flex().gap_2().flex_wrap();
                            for opt in options {
                                let chosen = pick.as_deref() == Some(opt.as_str());
                                let mid2 = msg_id.clone();
                                let opt2 = opt.clone();
                                let mut ob = div()
                                    .id(ElementId::Name(
                                        format!("uinput-{msg_id}-{qi}-{opt}").into(),
                                    ))
                                    .px(px(10.))
                                    .py(px(4.))
                                    .rounded(px(999.))
                                    .border_1()
                                    .cursor_pointer()
                                    .text_size(px(12.))
                                    .child(opt);
                                if chosen {
                                    ob = ob
                                        .border_color(theme.foreground)
                                        .bg(sel_fill)
                                        .text_color(theme.foreground);
                                } else {
                                    ob = ob
                                        .border_color(theme.border)
                                        .text_color(theme.muted_foreground)
                                        .hover(|d| d.bg(rgba(0x00000005)));
                                }
                                ob = ob.on_click(cx.listener(
                                    move |this, _ev, _w, cx| {
                                        this.user_input_draft.retain(|(id, i, _)| {
                                            !(id == &mid2 && *i == qi)
                                        });
                                        this.user_input_draft
                                            .push((mid2.clone(), qi, opt2.clone()));
                                        cx.notify();
                                    },
                                ));
                                orow = orow.child(ob);
                            }
                            qv = qv.child(orow);
                        }
                        card = card.child(qv);
                    }
                    // Footer: hint + Send (disabled until every question's
                    // effective pick exists).
                    let sendable = all_answerable;
                    let mut foot = h_flex().items_center().justify_between().child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child("The agent can keep working"),
                    );
                    if sendable {
                        let mid3 = msg_id.clone();
                        let qs: Vec<String> = questions
                            .iter()
                            .map(|q| str_at(q, "title").to_string())
                            .collect();
                        let answers: Vec<String> = questions
                            .iter()
                            .enumerate()
                            .map(|(qi, q)| {
                                self.user_input_draft
                                    .iter()
                                    .find(|(id, i, _)| id == &msg_id && *i == qi)
                                    .map(|(_, _, l)| l.clone())
                                    .or_else(|| {
                                        arr(q, "options")
                                            .first()
                                            .and_then(Value::as_str)
                                            .map(str::to_string)
                                    })
                                    .unwrap_or_default()
                            })
                            .collect();
                        foot = foot.child(
                            Button::new(ElementId::Name(
                                format!("uinput-send-{mid3}").into(),
                            ))
                            .primary()
                            .label("Send answer")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.respond_user_input(
                                    mid3.clone(),
                                    qs.clone(),
                                    answers.clone(),
                                    cx,
                                );
                            })),
                        );
                    } else {
                        foot = foot.child(
                            div()
                                .px(px(10.))
                                .py(px(4.))
                                .rounded(px(8.))
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground.opacity(0.5))
                                .child("Send answer"),
                        );
                    }
                    card = card.child(foot);
                }
                col = col.child(
                    h_flex().w_full().justify_center().child(
                        div().w_full().max_w(chat_mw).child(card),
                    ),
                );
                continue;
            }

            if text.trim().is_empty() {
                continue;
            }
            let is_match = !find_q.is_empty() && text.to_lowercase().contains(&find_q);

            if role == "user" {
                let mut pill = div()
                    .bg(user_bubble)
                    .rounded(px(16.))
                    .px_4()
                    .py(px(10.))
                    .max_w(px(560.));
                if is_match {
                    pill = pill.border_1().border_color(theme.warning);
                }
                // Upstream user-message footer: right-aligned timestamp
                // (hover-reveal) + copy / edit (pencil) / revert (undo).
                let mut ufooter = h_flex()
                    .mt(px(4.))
                    .gap(px(2.))
                    .items_center()
                    .justify_end();
                let uts = clock_time(str_at(m, "createdAt"));
                if !uts.is_empty() {
                    ufooter = ufooter.child(
                        div()
                            .mr(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground.opacity(0.45))
                            .child(uts),
                    );
                }
                let msg_id = str_at(m, "id").to_string();
                // Upstream revert target: the checkpoint of the turn the
                // next assistant message belongs to, minus one.
                let revert_tc: Option<i64> = (|| {
                    let turn_id = messages
                        .iter()
                        .skip(i + 1)
                        .find(|nm| str_at(nm, "role") == "assistant")
                        .map(|nm| str_at(nm, "turnId").to_string())
                        .filter(|t| !t.is_empty())?;
                    arr(thread_v, "checkpoints")
                        .iter()
                        .find(|c| str_at(c, "turnId") == turn_id)
                        .and_then(|c| {
                            c.get("checkpointTurnCount").and_then(Value::as_i64)
                        })
                        .map(|tc| (tc - 1).max(0))
                })();
                // Upstream shows the pencil only when
                // resolveTailUserMessageEditTarget would allow it: the
                // latest native-source user message whose tail spans at
                // most one turn (or carries the currently running turn).
                let msg_editable = {
                    let src = str_at(m, "source");
                    let native_src = src.is_empty() || src == "native";
                    let answered = messages.iter().any(|nm| {
                        str_at(get(get(nm, "asyncUserInput"), "response"), "messageId")
                            == msg_id
                    });
                    let latest = messages.iter().rposition(|nm| {
                        str_at(nm, "role") == "user" && {
                            let s = str_at(nm, "source");
                            s.is_empty() || s == "native"
                        }
                    });
                    let sess = get(detail, "session");
                    let active_turn = str_at(sess, "status") == "running"
                        && !str_at(sess, "activeTurnId").is_empty();
                    let mut tail: Vec<&str> = Vec::new();
                    for nm in &messages[i..] {
                        let t = str_at(nm, "turnId");
                        if !t.is_empty() && !tail.contains(&t) {
                            tail.push(t);
                        }
                    }
                    native_src
                        && !answered
                        && latest == Some(i)
                        && (tail.len() == 1 || (tail.is_empty() && active_turn))
                };
                let mut uicons: Vec<(&'static str, &'static [u8])> = vec![("umsg-copy", icons::COPY)];
                if msg_editable {
                    uicons.push(("umsg-edit", icons::NEW_THREAD));
                }
                if revert_tc.is_some() {
                    uicons.push(("umsg-revert", icons::UNDO));
                }
                for (iid, data) in uicons {
                    let msg_id2 = msg_id.clone();
                    let text2 = text.clone();
                    let tc = revert_tc;
                    ufooter = ufooter.child(
                        div()
                            .id(ElementId::Name(
                                format!("{iid}-{msg_id}").into(),
                            ))
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(
                                move |this, _ev, w, cx| match iid {
                                    "umsg-edit" => {
                                        this.editing_message_id =
                                            Some(msg_id2.clone());
                                        this.composer.update(cx, |s, cx| {
                                            s.set_value(text2.clone(), w, cx);
                                        });
                                        let f =
                                            this.composer.read(cx).focus_handle(cx);
                                        w.focus(&f, cx);
                                        cx.notify();
                                    }
                                    "umsg-revert" => {
                                        if let Some(tc) = tc {
                                            this.pending_revert =
                                                Some((msg_id2.clone(), tc));
                                            cx.notify();
                                        }
                                    }
                                    _ => {
                                        cx.write_to_clipboard(
                                            ClipboardItem::new_string(
                                                text2.clone(),
                                            ),
                                        );
                                    }
                                },
                            ))
                            .child(
                                svg_icon(data)
                                    .size(px(13.))
                                    .text_color(theme.muted_foreground.opacity(0.45)),
                            ),
                    );
                }
                col = col.child(
                    h_flex().w_full().justify_center().child(
                        h_flex().w_full().max_w(chat_mw).justify_end().child(
                            v_flex()
                                .items_end()
                                .child(pill.child(
                                    div().text_size(px(13.)).child(text),
                                ))
                                .child(ufooter),
                        ),
                    ),
                );
            } else {
                let mut wrap = div();
                if is_match {
                    wrap = wrap.border_l_2().border_color(theme.warning).pl_2();
                }
                // Footer row under a settled assistant message (upstream):
                // copy / fork / pin icon buttons followed by the wall-clock time.
                let mut footer = h_flex()
                    .mt(px(4.))
                    .gap(px(2.))
                    .items_center();
                let msg_id = str_at(m, "id").to_string();
                let pinned_ids: Vec<String> = arr(thread_v, "pinnedMessages")
                    .iter()
                    .map(|p| str_at(p, "messageId").to_string())
                    .collect();
                for (iid, data) in [
                    ("msg-copy", icons::COPY),
                    ("msg-fork", icons::BRANCH_SIMPLE),
                    ("msg-pin", icons::PIN),
                ] {
                    let msg_id2 = msg_id.clone();
                    let text2 = text.clone();
                    let is_pinned = pinned_ids.contains(&msg_id2);
                    footer = footer.child(
                        div()
                            .id(ElementId::Name(
                                format!("{iid}-{msg_id}").into(),
                            ))
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(
                                move |this, _ev, _w, cx| match iid {
                                    "msg-fork" => {
                                        if let Some(tid) =
                                            this.selected_thread.clone()
                                        {
                                            this.lineage(tid, false, None, cx);
                                        }
                                    }
                                    "msg-pin" => {
                                        if let Some(tid) =
                                            this.selected_thread.clone()
                                        {
                                            let op = if is_pinned {
                                                "thread.pinned-message.remove"
                                            } else {
                                                "thread.pinned-message.add"
                                            };
                                            let cell = this.client.clone();
                                            let mid = msg_id2.clone();
                                            cx.background_executor()
                                                .spawn(async move {
                                                    let Some(client) =
                                                        ensure_client(&cell)
                                                    else {
                                                        return;
                                                    };
                                                    client.call(
                                                        "orchestration.dispatchCommand",
                                                        json!({
                                                            "type": op,
                                                            "commandId": uuid(),
                                                            "threadId": tid,
                                                            "messageId": mid,
                                                            "createdAt": now_iso(),
                                                        }),
                                                    );
                                                })
                                                .detach();
                                        }
                                    }
                                    _ => {
                                        cx.write_to_clipboard(
                                            ClipboardItem::new_string(
                                                text2.clone(),
                                            ),
                                        );
                                    }
                                },
                            ))
                            .child(
                                svg_icon(data)
                                    .size(px(13.))
                                    .text_color(if is_pinned && iid == "msg-pin" {
                                        theme.foreground
                                    } else {
                                        theme.muted_foreground
                                    }),
                            ),
                    );
                }
                let ts = clock_time(str_at(m, "createdAt"));
                if !ts.is_empty() {
                    footer = footer.child(
                        div()
                            .ml(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(ts),
                    );
                }
                col = col.child(
                    h_flex().w_full().justify_center().child(
                        div().w_full().max_w(chat_mw).child(
                            v_flex()
                                .child(wrap.child(
                                    TextView::markdown(
                                        ElementId::Name(format!("md-{i}-{}", str_at(m, "id")).into()),
                                        text,
                                    )
                                    .style({
                                        let mut s =
                                            gpui_kit::component::text::TextViewStyle::default();
                                        s.code_block.padding.top = Some(px(28.).into());
                                        s.highlight_theme =
                                            gpui_kit::component::highlighter::HighlightTheme::default_dark();
                                        s
                                    })
                                    .code_block_actions(|cb, _w, _cx| {
                                        let lang = cb.lang().map(|l| l.to_string()).unwrap_or_default();
                                        h_flex()
                                            .w(px(700.))
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(rgb(0x6e7781))
                                                    .child(lang),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap(px(2.))
                                                    .child(
                                                        svg_icon(icons::COPY)
                                                            .size(px(14.))
                                                            .text_color(rgb(0x6e7781)),
                                                    )
                                                    .child(
                                                        svg_icon(icons::TEXT_WRAP)
                                                            .size(px(14.))
                                                            .text_color(rgb(0x6e7781)),
                                                    ),
                                            )
                                    })
                                    .selectable(true),
                                ))
                                .child(footer),
                        ),
                    ),
                );
            }
        }

        // Activities grouped in a collapsible tool-call card (web parity):
        // a single activity stays a plain row; several collapse into a card.
        let activities: Vec<Value> = arr(thread_v, "activities")
            .iter()
            .filter(|a| !str_at(a, "kind").is_empty() || !str_at(a, "summary").is_empty())
            .cloned()
            .collect();
        let activity_row = |a: &Value| {
            let kind = str_at(a, "kind");
            let summary = str_at(a, "summary");
            let color = match str_at(a, "tone") {
                "error" => theme.danger,
                _ => theme.muted_foreground,
            };
            let label = if summary.is_empty() {
                kind.to_string()
            } else if kind.is_empty() {
                summary.to_string()
            } else {
                format!("{kind} — {summary}")
            };
            h_flex()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("\u{25e6}"),
                )
                .child(div().text_xs().text_color(color).child(label))
        };
        if activities.len() == 1 {
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(activity_row(&activities[0])),
                ),
            );
        } else if activities.len() > 1 {
            let mut card = v_flex()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .p_2()
                .gap_1()
                .child(
                    h_flex()
                        .id("tools-card")
                        .cursor_pointer()
                        .gap_2()
                        .items_center()
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.tools_expanded = !this.tools_expanded;
                            cx.notify();
                        }))
                        .child(
                            Icon::new(AssetIcon::Wrench)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div().text_xs().font_weight(FontWeight::SEMIBOLD).child(format!(
                                "{} actions {}",
                                activities.len(),
                                if self.tools_expanded { "\u{25be}" } else { "\u{25b8}" }
                            )),
                        ),
                );
            if self.tools_expanded {
                for a in &activities {
                    card = card.child(activity_row(a));
                }
            }
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(card),
                ),
            );
        }

        // Active tools currently running.
        for t in arr(thread_v, "activeTools") {
            let item = str_at(t, "item_id");
            if item.is_empty() {
                continue;
            }
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        div()
                            .text_xs()
                            .text_color(theme.primary)
                            .child(format!("\u{25cf} tool running: {item}")),
                    ),
                ),
            );
        }

        // Proposed plans (rendered as markdown cards).
        for p in arr(thread_v, "proposedPlans") {
            let md = str_at(p, "plan_markdown");
            if md.trim().is_empty() {
                continue;
            }
            let implemented = !get(p, "implemented_at").is_null();
            let mut card = v_flex()
                .gap_1()
                .w_full()
                .bg(sel_fill)
                .rounded_md()
                .p_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.primary)
                        .child(if implemented {
                            "PROPOSED PLAN (implemented)".to_string()
                        } else {
                            "PROPOSED PLAN".to_string()
                        }),
                );
            card = card.child(
                TextView::markdown(
                    ElementId::Name(format!("plan-{}", str_at(p, "id")).into()),
                    md.to_string(),
                )
                .style({
                    let mut s = gpui_kit::component::text::TextViewStyle::default();
                    s.highlight_theme =
                        gpui_kit::component::highlighter::HighlightTheme::default_dark();
                    s
                })
                .selectable(true),
            );
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(card),
                ),
            );
        }

        // Pending interactions (approvals awaiting a decision).
        for p in arr(thread_v, "pendingInteractions") {
            if str_at(p, "status") != "pending" {
                continue;
            }
            let kind = str_at(p, "interactionKind").replace('_', " ");
            let rid = str_at(p, "requestId").to_string();
            let mut row_el = h_flex().gap_2().child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning)
                    .child(format!("approval pending: {kind}")),
            );
            if !rid.is_empty() {
                for (label, decision) in [("Approve", "accept"), ("Decline", "decline")] {
                    let r = rid.clone();
                    row_el = row_el.child(
                        Button::new(ElementId::Name(
                            format!("approve-{decision}-{rid}").into(),
                        ))
                        .ghost()
                        .label(label)
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.respond_approval(r.clone(), decision, cx)
                        })),
                    );
                }
            }
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(row_el),
                ),
            );
        }

        col
    }

    fn open_split_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        let old = self.split_stream_id.take();
        self.split_thread = Some(thread_id.clone());
        self.focused_pane = 1;
        self.split_detail = None;
        let client = self.client.clone();
        let tid = thread_id;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            if let Some(old) = old {
                client.interrupt(&old);
            }
            let payload = json!({"threadId": tid});
            let c = client.clone();
            let p = payload.clone();
            let detail = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.split_thread.as_deref() == Some(tid.as_str()) {
                    this.split_detail = detail;
                }
                cx.notify();
            });
            // Live transcript via subscribeThread (same pattern as the
            // primary pane): each chunk triggers a detail refetch.
            let c = client.clone();
            let (rid, frames) = match c.request("orchestration.subscribeThread", payload.clone()) {
                Some(v) => v,
                None => return,
            };
            let rid_for_ui = rid.clone();
            let _ = this.update(cx, |this: &mut Self, _| {
                this.split_stream_id = Some(rid_for_ui);
            });
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                    c.finish_request(&rid);
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let p = payload.clone();
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if this.split_thread.as_deref() == Some(tid.as_str()) {
                        this.split_detail = d;
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn close_split(&mut self, cx: &mut Context<Self>) {
        if let Some(rid) = self.split_stream_id.take() {
            let cell = self.client.clone();
            cx.background_executor()
                .spawn(async move {
                    if let Some(c) = ensure_client(&cell) {
                        c.interrupt(&rid);
                    }
                })
                .detach();
        }
        self.split_open = false;
        self.split_thread = None;
        self.split_detail = None;
        self.focused_pane = 0;
        cx.notify();
    }

    fn send_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.split_composer.clone() else {
            return;
        };
        let text = input.read(cx).value().trim().to_string();
        let Some(tid) = self.split_thread.clone() else {
            return;
        };
        if text.is_empty() || self.sending {
            return;
        }
        self.sending = true;
        input.update(cx, |s, cx| s.clean(window, cx));
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.sending = false;
                    cx.notify();
                });
                return;
            };
            let cmd = json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "runtimeMode": mode,
                "interactionMode": imode,
                "createdAt": now_iso(),
            });
            let c = client.clone();
            let _ = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                .await;
            let p = json!({"threadId": tid});
            let c = client.clone();
            let d = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.sending = false;
                if let Some(d) = d {
                    this.split_detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        if text.is_empty() || self.sending {
            return;
        }
        // Edit & resend (upstream thread.message.edit-and-resend): the pencil
        // on a user message reloads it into the composer; sending replaces
        // the original turn instead of appending a new one.
        if let Some(edit_id) = self.editing_message_id.take() {
            if self.selected_thread.is_some() {
                let tid = self.selected_thread.clone().unwrap_or_default();
                self.sending = true;
                self.composer.update(cx, |s, cx| s.clean(window, cx));
                let cell = self.client.clone();
                let mode = self.runtime_mode;
                let imode = self.interaction_mode;
                cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let cell2 = cell.clone();
                    let client = cx
                        .background_executor()
                        .spawn(async move { ensure_client(&cell2) })
                        .await;
                    let Some(client) = client else {
                        let _ = this.update(cx, |this: &mut Self, cx| {
                            this.sending = false;
                            cx.notify();
                        });
                        return;
                    };
                    let cmd = json!({
                        "type": "thread.message.edit-and-resend",
                        "commandId": uuid(),
                        "threadId": tid,
                        "messageId": edit_id,
                        "text": text,
                        "runtimeMode": mode,
                        "interactionMode": imode,
                        "createdAt": now_iso(),
                    });
                    let c = client.clone();
                    let _ = cx
                        .background_executor()
                        .spawn(async move {
                            c.call("orchestration.dispatchCommand", cmd)
                        })
                        .await;
                    let p = json!({"threadId": tid});
                    let c = client.clone();
                    let d = cx
                        .background_executor()
                        .spawn(async move {
                            c.call("orchestration.getThreadDetailSnapshot", p)
                        })
                        .await;
                    let _ = this.update(cx, |this: &mut Self, cx| {
                        this.sending = false;
                        if let Some(d) = d {
                            this.detail = Some(d);
                        }
                        cx.notify();
                    });
                })
                .detach();
                return;
            }
        }
        // While a turn is live, upstream enqueues the follow-up instead of
        // sending it (the queued pills render above the composer).
        if self.selected_thread.is_some()
            && matches!(session_phase(self.detail.as_ref()), "running" | "connecting")
        {
            self.queued_turns.push(text);
            self.composer.update(cx, |s, cx| s.clean(window, cx));
            cx.notify();
            return;
        }
        self.sending = true;
        self.composer.update(cx, |s, cx| s.clean(window, cx));
        let selected = self.selected_thread.clone();
        let project_id = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        // Attachments uploaded earlier keep the thread id they were staged
        // under (a fresh thread adopts it as its id).
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let attachments = std::mem::take(&mut self.pending_attachments);
        let att_thread = if self.selected_thread.is_some() {
            self.selected_thread.clone()
        } else {
            self.composer_thread_id.clone()
        };
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                eprintln!("native: send: no ws client");
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.sending = false;
                    cx.notify();
                });
                return;
            };
            let model_selection = json!({
                "provider": "codex",
                "instanceId": "codex",
                "model": "gpt-6-astra",
            });
            let tid = selected.clone().unwrap_or_else(|| {
                att_thread.clone().unwrap_or_else(uuid)
            });
            let mut cmds = Vec::new();
            if selected.is_none() {
                if let Some(pid) = project_id {
                    cmds.push(json!({
                        "type": "thread.create",
                        "commandId": uuid(),
                        "threadId": tid,
                        "projectId": pid,
                        "title": text.chars().take(60).collect::<String>(),
                        "modelSelection": model_selection,
                        "runtimeMode": mode,
                        "interactionMode": imode,
                        "envMode": "local",
                        "branch": null,
                        "worktreePath": null,
                        "createdAt": now_iso(),
                    }));
                }
            }
            cmds.push(json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": attachments,
                },
                "runtimeMode": mode,
                "interactionMode": imode,
                "createdAt": now_iso(),
            }));
            for cmd in cmds {
                let c = client.clone();
                let r = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                    .await;
                if r.is_none() {
                    eprintln!("native: dispatchCommand failed");
                } else {
                }
            }
            // Refresh snapshot + detail.
            let c = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                .await;
            let p = json!({"threadId": tid});
            let c = client.clone();
            let d = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.sending = false;
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                this.selected_thread = Some(tid.clone());
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for Native {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn row(
    id: impl Into<ElementId>,
    title: String,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Native>,
    on_click: impl Fn(&mut Native, &ClickEvent, &mut Window, &mut Context<Native>) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    let mut el = v_flex()
        .id(id)
        .w_full()
        .px_3()
        .py_2()
        .gap_1()
        .cursor_pointer()
        .rounded_md()
        .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(title))
        .child(div().text_xs().text_color(theme.muted_foreground).child(subtitle));
    if selected {
        let dark_ui = matches!(
            cx.window_appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        el = el.bg(if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        });
    }
    el.on_click(cx.listener(on_click))
}

impl Render for Native {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        // Dark-aware chrome colors: upstream paints the top strip, rail well
        // and hairlines from the theme, not fixed light grays.
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let well_bg = if dark_ui { rgb(0x1d1d1d) } else { rgb(0xf7f7f7) };
        let hairline = if dark_ui { rgb(0x2e2e2e) } else { rgb(0xe5e5e5) };
        let subtle = if dark_ui { rgba(0xffffff14) } else { rgba(0x0000000d) };
        let chip_bg = if dark_ui { rgba(0xffffff12) } else { rgba(0x00000008) };
        let soft_bg: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            rgb(0xf7f7f7).into()
        };
        // Upstream selection fills are neutral, not gpui's navy list_active.
        let sel_fill: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        };
        let content_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let sidebar_bg: gpui::Hsla = if dark_ui {
            rgb(0x151515).into()
        } else {
            theme.background
        };
        let popover_bg: gpui::Hsla = if dark_ui {
            rgb(0x161616).into()
        } else {
            theme.popover
        };

        // ---- sidebar ----
        let mut sidebar = v_flex()
            .w_full()
            .h_full()
            .gap_0p5()
            .px_2()
            .pb_2()
            // Card interior below the top strip — the first row lands ~4px
            // under the card's top edge like the upstream panel.
            .pt(px(4.))
            .overflow_y_scrollbar();
        let query = self.search.read(cx).value().to_string().to_lowercase();
        // Header: bold wordmark + chevron (workspace switcher), search +
        // new-thread icon buttons on the right — mirrors the web sidebar.
        sidebar = sidebar.child(
            h_flex()
                .items_center()
                .gap_1()
                .mb_1()
                .child(
                    div()
                        .px_1()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Synara"),
                )
                .child(
                    // Upstream surface picker: active surface ("Synara") plus
                    // "Hubs", each with its subtitle. Selecting Hubs opens the
                    // hubs rail section.
                    Button::new("app-menu")
                        .ghost()
                        .compact()
                        .icon(AssetIcon::ChevronDown)
                        .dropdown_menu({
                            let weak = cx.weak_entity();
                            move |menu, _, _cx| {
                                let w2 = weak.clone();
                                menu.min_w(px(200.))
                                    .item(
                                        PopupMenuItem::element(|_, _| {
                                            v_flex()
                                                .gap_0p5()
                                                .child(div().text_size(px(13.)).child("Synara"))
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgba(0x777777aa))
                                                        .child("Build, debug, and ship"),
                                                )
                                        })
                                        .checked(true)
                                        .on_click(|_, _, _| {}),
                                    )
                                    .item(
                                        PopupMenuItem::element(|_, _| {
                                            v_flex()
                                                .gap_0p5()
                                                .child(div().text_size(px(13.)).child("Hubs"))
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgba(0x777777aa))
                                                        .child("Coordinated work across repos"),
                                                )
                                        })
                                        .on_click(move |_, _w, cx| {
                                            let _ = w2.update(cx, |this, cx| {
                                                this.clear_views();
                                                this.view_studio = true;
                                                cx.notify();
                                            });
                                        }),
                                    )
                            }
                        }),
                )
                .child(div().flex_1())
                .child(
                    icon_btn_pad("sidebar-search", icons::SEARCH, 18., 8., theme.secondary, theme.muted_foreground)
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.search_open = !this.search_open;
                            cx.notify();
                        })),
                )
                .child(
                    icon_btn_pad("sidebar-new", icons::NEW_THREAD, 18., 8., theme.secondary, theme.muted_foreground)
                        .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
                ),
        );
        if self.search_open {
            sidebar = sidebar.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.search))),
            );
        }
        sidebar = sidebar.child(
            h_flex()
                .id("new-thread")
                .w_full()
                .h(px(32.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .child(
                    svg_icon(icons::NEW_THREAD)
                        .size(px(16.))
                        .text_color(theme.foreground),
                )
                .child(div().text_size(px(13.)).child("New thread"))
                .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
        );
        sidebar = sidebar.child(
            div()
                .mt(px(15.))
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(if self.view_spaces {
                    // spaces — upstream navigates INTO the space: same sidebar
                    // chrome, the section label is the space name.
                    arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "spaces")
                        .first()
                        .map(|s| str_at(s, "name").to_string())
                        .filter(|n: &String| !n.is_empty())
                        .unwrap_or_else(|| "Void".to_string())
                } else {
                    "Projects".to_string()
                }),
        );
        if arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects").is_empty() {
            sidebar = sidebar.child(
                div()
                    .w_full()
                    .py_2()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(div().text_center().child("No projects yet")),
            );
        }
        let all_threads: Vec<Value> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads").to_vec();
        let live_thread = |t: &Value| {
            get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null()
        };
        let matches_query = |t: &Value| {
            query.is_empty()
                || str_at(t, "title").to_lowercase().contains(&query)
                || str_at(t, "latestReplyPreview")
                    .to_lowercase()
                    .contains(&query)
        };
        let pinned_ids = self.pinned_threads.clone();
        let selected_tid = self.selected_thread.clone();
        let thread_row = |thread: &Value,
                          theme: &Theme,
                          pinned: &[String],
                          selected: Option<&str>,
                          indent: f32,
                          cx: &mut Context<Self>|
         -> Stateful<Div> {
            let tid = str_at(thread, "id").to_string();
            let title = str_at(thread, "title");
            let is_pinned = pinned.contains(&tid);
            let is_sel = selected == Some(tid.as_str());
            // Upstream: the row's leading glyph is the thread's provider icon
            // (session.providerName), not a uniform marker.
            let provider_icon = provider_icon(str_at(get(thread, "session"), "providerName"));
            let mut el = h_flex()
                .id(ElementId::Name(format!("thread-{tid}").into()))
                .w_full()
                .h(px(28. * self.density_scale()))
                .pl(px(indent))
                .pr_2()
                .gap(px(6.))
                .items_center()
                .cursor_pointer()
                .rounded_md()
                .child(
                    svg_icon(provider_icon)
                        .size(px(12.))
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_size(px(13.))
                        .whitespace_nowrap()
                        .text_color(if is_sel {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(if title.is_empty() {
                            "(untitled)".to_string()
                        } else {
                            title.to_string()
                        }),
                );
            // Upstream ThreadRowQuickActions: hovering a row reveals
            // pin / archive / done icon buttons at its trailing edge.
            let hovered = self.hover_thread.as_deref() == Some(tid.as_str());
            let settled = !str_at(thread, "settledAt").is_empty();
            if hovered {
                let mut qa = h_flex().items_center().gap_1();
                for (aid, icon, act) in [
                    ("qa-pin", if is_pinned { icons::PIN_FILL } else { icons::PIN }, "pin"),
                    ("qa-archive", icons::S_ARCHIVE, "archive"),
                    ("qa-done", if settled { icons::UNDO } else { icons::CHECK }, "done"),
                ] {
                    let tid_q = tid.clone();
                    qa = qa.child(
                        div()
                            .id(ElementId::Name(format!("{aid}-{tid_q}").into()))
                            .size(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.muted.opacity(0.35)))
                            .child(
                                svg_icon(icon)
                                    .size(px(11.))
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                cx.stop_propagation();
                                match act {
                                    "pin" => this.toggle_pin(tid_q.clone(), cx),
                                    "archive" => {
                                        let c = tid_q.clone();
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.archive",
                                                "commandId": uuid(),
                                                "threadId": c,
                                            }),
                                            cx,
                                        );
                                    }
                                    _ => {
                                        let c = tid_q.clone();
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.meta.update",
                                                "commandId": uuid(),
                                                "threadId": c,
                                                "isSettled": !settled,
                                            }),
                                            cx,
                                        );
                                    }
                                }
                            })),
                    );
                }
                el = el.child(qa);
            } else {
                // Upstream trailing slot: status glyph (SidebarStatusTrailingGlyph)
                // then the pin affordance. Completed -> accent dot, Reminder ->
                // clock, approval/input/plan -> colored status dot.
                if let Some(k) = inbox_needs_you_kind(
                    thread,
                    &self.dismissed_status,
                    &self.visited_threads,
                ) {
                    let label = needs_you_label(thread, k);
                    el = el.child(if label == "Reminder" {
                        svg_icon(icons::CLOCK)
                            .size(px(11.))
                            .text_color(rgb(0x38bdf8))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(7.))
                            .rounded_full()
                            .bg(match label {
                                "Pending Approval" => theme.warning,
                                "Awaiting Input" => rgb(0x6366f1).into(),
                                "Plan Ready" => rgb(0x8b5cf6).into(),
                                _ => theme.accent,
                            })
                            .into_any_element()
                    });
                }
                if is_pinned {
                    el = el.child(
                        svg_icon(icons::PIN_FILL)
                            .size(px(11.))
                            .text_color(theme.muted_foreground),
                    );
                }
            }
            if is_sel {
                el = el.bg(sel_fill);
            } else {
                el = el.hover(|d| d.bg(theme.secondary));
            }
            let tid_ctx = tid.clone();
            let tid_drag = tid.clone();
            let title_drag = if title.is_empty() {
                "(untitled)".to_string()
            } else {
                title.to_string()
            };
            let tid_click = tid.clone();
            el = el
                .on_click(cx.listener(move |this, _ev, _w, cx| {
                    this.ctx_menu = None;
                    this.ctx_sub = None;
                    this.ctx_confirm_delete = None;
                    this.open_thread(tid_click.clone(), cx)
                }))
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                        this.ctx_menu = Some((
                            tid_ctx.clone(),
                            ev.position.x.as_f32(),
                            ev.position.y.as_f32(),
                        ));
                        this.ctx_sub = None;
                        this.ctx_confirm_delete = None;
                        cx.notify();
                        cx.stop_propagation();
                    }),
                );
            // Upstream hover: quick-action icons appear and a delayed hover
            // card shows title/status/project/model.
            let tid_hover = tid.clone();
            let tid_hover2 = tid.clone();
            el = el.on_hover(cx.listener(move |this, hovered: &bool, w, cx| {
                if *hovered {
                    if this.hover_thread.as_deref() != Some(tid_hover.as_str()) {
                        this.hover_thread = Some(tid_hover.clone());
                        cx.notify();
                    }
                    // Upstream hover card: short delay, then info popup.
                    this.hover_card_seq += 1;
                    let seq = this.hover_card_seq;
                    let pos = w.mouse_position();
                    let (px_, py_) = (f32::from(pos.x), f32::from(pos.y));
                    let tidc = tid_hover2.clone();
                    cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(450))
                            .await;
                        let _ = this.update(cx, |this: &mut Self, cx| {
                            if seq == this.hover_card_seq
                                && this.hover_thread.as_deref() == Some(tidc.as_str())
                            {
                                this.hover_card = Some((tidc.clone(), px_, py_));
                                cx.notify();
                            }
                        });
                    })
                    .detach();
                } else if this.hover_thread.as_deref() == Some(tid_hover.as_str()) {
                    this.hover_thread = None;
                    this.hover_card_seq += 1;
                    if this.hover_card.as_ref().map(|c| c.0.as_str()) == Some(tid_hover.as_str()) {
                        this.hover_card = None;
                    }
                    cx.notify();
                }
            }));
            // Upstream beginThreadDrag: the row drags onto the composer
            // as an @mention (and onto chat panes as a split).
            el.on_drag(
                ThreadDrag(tid_drag.clone()),
                move |_d, _pos, _w, cx| cx.new(|_| DragGhost(title_drag.clone())),
            )
        };
        // Projects: only real (non-chat) projects appear here — chat-kind
        // projects (Home) surface only via the "Chats" section. Pinned first,
        // then creation order, matching the web sidebar.
        let mut projects_list: Vec<Value> = arr(
            self.snapshot.as_ref().unwrap_or(&Value::Null),
            "projects",
        )
        .iter()
        .filter(|p| str_at(p, "kind") != "chat")
        .cloned()
        .collect();
        projects_list.sort_by_key(|p| {
            (
                if get(p, "isPinned").as_bool().unwrap_or(false) {
                    0
                } else {
                    1
                },
                str_at(p, "createdAt").to_string(),
            )
        });
        for (project_index, project) in projects_list.iter().enumerate() {
            let pid = str_at(project, "id").to_string();
            // Expanded when it has visible threads nested below.
            let mut kids: Vec<Value> = all_threads
                .iter()
                .filter(|t| str_at(t, "projectId") == pid && live_thread(t) && matches_query(t))
                .cloned()
                .collect();
            // Newest threads first — the web sidebar lists latest activity on top.
            kids.sort_by_key(|t| {
                (
                    if pinned_ids.contains(&str_at(t, "id").to_string()) {
                        0
                    } else {
                        1
                    },
                    std::cmp::Reverse(str_at(t, "createdAt").to_string()),
                )
            });
            let expanded = !kids.is_empty() && !self.collapsed_projects.contains(&pid);
            let pinned_proj = get(project, "isPinned").as_bool().unwrap_or(false);
            // A project whose workspace carries a favicon (here: the synara
            // checkout) shows its logo in a hairline frame; the pin glyph wins
            // for pinned projects; otherwise the folder opens/closes.
            let has_logo = str_at(project, "workspaceRoot").ends_with("synara");
            let glyph = if pinned_proj {
                icons::PIN_FILL
            } else if has_logo {
                icons::LOGO
            } else if expanded {
                icons::FOLDER_OPEN
            } else {
                icons::FOLDER
            };
            let glyph_el = if has_logo && !pinned_proj {
                div()
                    .size(px(19.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .border_1()
                    .border_color(theme.border)
                    .child(svg_icon(glyph).size(px(12.)).text_color(theme.foreground))
                    .into_any_element()
            } else {
                svg_icon(glyph)
                    .size(px(16.))
                    .text_color(theme.muted_foreground)
                    .into_any_element()
            };
            // The disclosure chevron only appears beside the title when the
            // leading glyph cannot convey open/closed (pin or logo).
            let needs_chevron = pinned_proj || has_logo;
            let is_active_project = has_logo;
            let mut row_head = h_flex()
                .gap_1p5()
                .items_center()
                .h(px(26.))
                .child(glyph_el)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(13.))
                        .child(str_at(project, "title").to_string()),
                );
            if needs_chevron {
                row_head = row_head.child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.muted_foreground)
                        .child(if expanded { "\u{2228}" } else { "\u{203a}" }),
                );
            }
            row_head = row_head.child(div().flex_1());
            if is_active_project {
                row_head = row_head.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(gpui_kit::rgb(0x22c55e)),
                );
            }
            let pid_ctx = pid.clone();
            let pid_hover = pid.clone();
            let pid_hover2 = pid.clone();
            let proj_head = v_flex()
                .gap_0()
                .mt(if project_index == 0 { px(8.) } else { px(14.) })
                .id(ElementId::Name(
                    format!("project-{}", str_at(project, "id")).into(),
                ))
                .cursor_pointer()
                .rounded_md()
                .px_2()
                .child(row_head)
                .on_click(cx.listener(move |this, _ev, _w, cx| {
                    this.proj_ctx_menu = None;
                    this.proj_ctx_confirm = false;
                    // Upstream toggleProject: the row click expands/collapses
                    // the folder's thread list.
                    if this.collapsed_projects.contains(&pid) {
                        this.collapsed_projects.remove(&pid);
                    } else {
                        this.collapsed_projects.insert(pid.clone());
                    }
                    this.view_files = false;
                    cx.notify();
                }))
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                        this.proj_ctx_menu = Some((
                            pid_ctx.clone(),
                            ev.position.x.as_f32(),
                            ev.position.y.as_f32(),
                        ));
                        this.proj_ctx_confirm = false;
                        // The menu replaces the hover card (upstream's
                        // PreviewCard closes as the menu opens).
                        this.hover_project = None;
                        this.project_card = None;
                        this.project_card_seq += 1;
                        cx.notify();
                        cx.stop_propagation();
                    }),
                )
                .on_hover(cx.listener(move |this, hovered: &bool, w, cx| {
                    if *hovered {
                        if this.hover_project.as_deref() != Some(pid_hover.as_str()) {
                            this.hover_project = Some(pid_hover.clone());
                            cx.notify();
                        }
                        // Upstream PreviewCard: short delay, then info popup.
                        this.project_card_seq += 1;
                        let seq = this.project_card_seq;
                        let pos = w.mouse_position();
                        let (px_, py_) = (f32::from(pos.x), f32::from(pos.y));
                        let pidc = pid_hover2.clone();
                        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(450))
                                .await;
                            let _ = this.update(cx, |this: &mut Self, cx| {
                                if seq == this.project_card_seq
                                    && this.hover_project.as_deref() == Some(pidc.as_str())
                                {
                                    this.project_card = Some((pidc.clone(), px_, py_));
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                    } else if this.hover_project.as_deref() == Some(pid_hover.as_str()) {
                        this.hover_project = None;
                        this.project_card_seq += 1;
                        if this.project_card.as_ref().map(|c| c.0.as_str())
                            == Some(pid_hover.as_str())
                        {
                            this.project_card = None;
                        }
                        cx.notify();
                    }
                }));
            sidebar = sidebar.child(proj_head);
            if !expanded {
                continue;
            }
            for thread in &kids {
                sidebar = sidebar.child(thread_row(
                    thread,
                    &theme,
                    &pinned_ids,
                    selected_tid.as_deref(),
                    34.,
                    cx,
                ));
            }
        }
        sidebar = sidebar.child(
            div()
                .id("chats-toggle")
                .mt_2()
                .cursor_pointer()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(if self.chats_collapsed {
                    "Chats \u{203a}"
                } else {
                    "Chats \u{2228}"
                })
                .on_click(cx.listener(|this, _ev, _w, cx| {
                    this.chats_collapsed = !this.chats_collapsed;
                    cx.notify();
                })),
        );
        // "Chats" = threads filed under chat-kind projects (Home), matching
        // the web sidebar's home chats section.
        let chat_project_ids: Vec<String> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
        let mut chat_threads: Vec<Value> = if self.chats_collapsed {
            Vec::new()
        } else {
            all_threads
                .iter()
                .filter(|t| {
                    chat_project_ids.iter().any(|id| id == str_at(t, "projectId"))
                        && live_thread(t)
                        && matches_query(t)
                })
                .cloned()
                .collect()
        };
        chat_threads.sort_by_key(|t| {
            if pinned_ids.contains(&str_at(t, "id").to_string()) { 0 } else { 1 }
        });
        for thread in &chat_threads {
            sidebar = sidebar.child(thread_row(
                thread,
                &theme,
                &pinned_ids,
                selected_tid.as_deref(),
                8.,
                cx,
            ));
        }
        // Server-side content matches (searchThreads).
        if !self.search_hits.is_empty() {
            sidebar = sidebar.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child("CONTENT MATCHES"),
            );
            for hit in self.search_hits.clone() {
                let tid = str_at(&hit, "threadId").to_string();
                let count = get(&hit, "matchCount").as_i64().unwrap_or(0);
                let excerpt = str_at(&hit, "excerpt");
                // Show the thread's title when we know it.
                let known = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .find(|t| str_at(t, "id") == tid)
                    .map(|t| str_at(t, "title").to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| tid.chars().take(8).collect());
                sidebar = sidebar.child(row(
                    ElementId::Name(format!("hit-{tid}-{count}").into()),
                    format!("{known} (x{count})"),
                    excerpt.chars().take(80).collect(),
                    false,
                    cx,
                    move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                ));
            }
        }
        // ---- SidebarFooter: DebugFeatureFlagsMenu (upstream dev tool) ----
        // The flyout opens upward above the footer row, like upstream's
        // ComposerPickerMenuPopup side="top".
        {
            const FLAG_ROWS: [(&'static str, &'static str); 3] = [
                (
                    "Keep debug error toasts open",
                    "Disable auto-dismiss for locally triggered error toasts.",
                ),
                (
                    "Pin git progress toast",
                    "Keep a looping git progress toast visible for styling.",
                ),
                (
                    "Show debug task banner",
                    "Render a local sample active task banner for UI testing.",
                ),
            ];
            let mut footer = v_flex().mt_auto();
            if self.flags_menu_open {
                let mut menu = v_flex()
                    .mx(px(4.))
                    .mb(px(4.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg()
                    .py(px(6.));
                menu = menu.child(
                    div()
                        .px(px(10.))
                        .pt(px(4.))
                        .pb(px(6.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child("Local feature flags"),
                );
                // Action row — fires stacked Git-failure toasts.
                menu = menu.child(
                    div()
                        .id("ff-trigger-toasts")
                        .mx(px(4.))
                        .px(px(8.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.foreground)
                                .child("Trigger action failed toasts"),
                        )
                        .child(
                            div()
                                .mt(px(1.))
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(
                                    "Show stacked Git action failure toasts for local UI testing.",
                                ),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.trigger_debug_toasts(cx);
                        })),
                );
                for (i, (label, desc)) in FLAG_ROWS.iter().enumerate() {
                    let on = self.feature_flags[i];
                    let mut track = div()
                        .w(px(28.))
                        .h(px(16.))
                        .rounded_full()
                        .flex()
                        .items_center();
                    track = if on {
                        track.bg(rgb(0xc74614)).justify_end()
                    } else {
                        track.bg(rgb(0xd9d9d9)).justify_start()
                    };
                    menu = menu.child(
                        div()
                            .id(ElementId::Name(format!("ff-row-{i}").into()))
                            .mx(px(4.))
                            .px(px(8.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_color(theme.foreground)
                                                    .child(*label),
                                            )
                                            .child(
                                                div()
                                                    .mt(px(1.))
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(track.child(
                                        div()
                                            .w(px(12.))
                                            .h(px(12.))
                                            .rounded_full()
                                            .bg(gpui::white())
                                            .mx(px(2.)),
                                    )),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.toggle_feature_flag(i, cx);
                            })),
                    );
                }
                menu = menu.child(
                    div()
                        .mt(px(4.))
                        .border_t_1()
                        .border_color(theme.border)
                        .px(px(10.))
                        .pt(px(6.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .child("Stored only in this browser profile."),
                );
                footer = footer.child(menu);
            }
            footer = footer.child(
                h_flex()
                    .id("ff-footer")
                    .mx(px(4.))
                    .px(px(8.))
                    .h(px(32.))
                    .gap(px(10.))
                    .items_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .child(
                        svg_icon(icons::FLAG)
                            .size(px(15.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Feature flags"),
                    )
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.flags_menu_open = !this.flags_menu_open;
                        cx.notify();
                    })),
            );
            sidebar = sidebar.child(footer);
        }
        // ---- transcript ----
        let mut pane = v_flex().flex_1().h_full().gap_2().on_drag_move::<ThreadDrag>(
            cx.listener(|this, _ev: &gpui::DragMoveEvent<ThreadDrag>, _w, cx| {
                if this.composer_thread_over {
                    this.composer_thread_over = false;
                    cx.notify();
                }
            }),
        );
        // Upstream shows the new-chat landing inside the thread pane when the
        // thread has no messages yet (same logo + heading + watermark + banner).
        let thread_msgs_empty = self.selected_thread.is_some()
            && arr(
                get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
                "messages",
            )
            .is_empty();
        let landing_title: Option<String> = if self.selected_thread.is_none() {
            Some("What should we do in this folder?".to_string())
        } else if thread_msgs_empty {
            let tid = self.selected_thread.as_deref().unwrap_or("");
            let snap = self.snapshot.as_ref().unwrap_or(&Value::Null);
            let pid = arr(snap, "threads")
                .iter()
                .find(|t| str_at(t, "id") == tid)
                .map(|t| str_at(t, "projectId").to_string())
                .unwrap_or_default();
            let name = arr(snap, "projects")
                .iter()
                .find(|p| str_at(p, "id") == pid)
                .map(|p| str_at(p, "title").to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "this folder".to_string());
            Some(format!("What should we do in {name}?"))
        } else {
            None
        };
        let show_landing = landing_title.is_some();
        // Landing unmounted: the next landing gets a fresh field (upstream
        // remounts AsciiWaveBackground each time).
        if !show_landing {
            self.ascii_fresh = true;
        }
        if self.selected_thread.is_some() && !show_landing {
            pane = pane.p_4();
        }
        if show_landing {
            // AsciiWaveBackground parity: animated glyph field — three
            // cycling noise patterns with crossfade, per-cell scramble,
            // center fade, and accent ripples on click. Three alpha layers
            // (dim ink, bright ink, accent) are stacked absolutely; a cell
            // contributes its glyph to exactly one layer per frame.
            if self.ascii_fresh || self.ascii_cells.len() != ASCII_COLS * ASCII_ROWS {
                self.ascii_t0 = std::time::Instant::now();
                self.ascii_seed = {
                    let n = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
                        .unwrap_or(0);
                    (n % 1000) as f32
                };
                let mut cells = vec![0u8; ASCII_COLS * ASCII_ROWS];
                let mut rng = (self.ascii_seed as u32) ^ 0x9e37_79b9;
                for cell in cells.iter_mut() {
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    *cell = (rng as usize % ASCII_GLYPHS.len()) as u8;
                }
                self.ascii_cells = cells;
                self.ascii_ripples.clear();
                self.ascii_fresh = false;
            }
            let now = std::time::Instant::now();
            let elapsed_ms =
                now.saturating_duration_since(self.ascii_t0).as_millis() as u64;
            let t = elapsed_ms as f32 / 1000.0 + self.ascii_seed;
            let reveal = smoothstep(0., ASCII_REVEAL_MS as f32, elapsed_ms as f32);
            let cycle = (elapsed_ms / ASCII_PATTERN_MS) as usize;
            let mix = smoothstep(
                (ASCII_PATTERN_MS - ASCII_CROSSFADE_MS) as f32,
                ASCII_PATTERN_MS as f32,
                (elapsed_ms % ASCII_PATTERN_MS) as f32,
            );
            let pat = &ASCII_PATTERNS[cycle % ASCII_PATTERNS.len()];
            let next_pat = if mix > 0. {
                Some(&ASCII_PATTERNS[(cycle + 1) % ASCII_PATTERNS.len()])
            } else {
                None
            };
            let fw = ASCII_COLS as f32 * ASCII_CELL_W;
            let fh = ASCII_ROWS as f32 * ASCII_CELL_H;
            let (fcx, fcy) = (fw / 2., fh / 2.);
            self.ascii_ripples.retain(|r| {
                now.saturating_duration_since(r.4).as_millis()
                    < ASCII_RIPPLE_LIFE_MS as u128
            });
            let ripples = self.ascii_ripples.clone();
            let mut rows_dim: Vec<String> = Vec::with_capacity(ASCII_ROWS);
            let mut rows_bright: Vec<String> = Vec::with_capacity(ASCII_ROWS);
            let mut rows_accent: Vec<String> = Vec::with_capacity(ASCII_ROWS);
            let mut rng = (elapsed_ms as u32)
                .wrapping_mul(747_796_405)
                .wrapping_add(2_891_336_453);
            for r in 0..ASCII_ROWS {
                let mut dim = String::with_capacity(ASCII_COLS);
                let mut bright = String::with_capacity(ASCII_COLS);
                let mut acc = String::with_capacity(ASCII_COLS);
                for c in 0..ASCII_COLS {
                    let idx = r * ASCII_COLS + c;
                    let x = (c as f32 + 0.5) * ASCII_CELL_W;
                    let y = (r as f32 + 0.5) * ASCII_CELL_H;
                    let (mut patch, mut wave) = ascii_sample(pat, x, y, fcx, fcy, t);
                    if let Some(np) = next_pat {
                        let (np_patch, np_wave) = ascii_sample(np, x, y, fcx, fcy, t);
                        patch += (np_patch - patch) * mix;
                        wave += (np_wave - wave) * mix;
                    }
                    let dx = (x - fcx) / (fw * 0.3);
                    let dy = (y - fcy) / (fh * 0.3);
                    let center_fade = (0.45 + 0.55 * (dx * dx + dy * dy).sqrt()).min(1.);
                    let ink_alpha =
                        0.18 * patch * patch * (0.4 + 0.6 * wave) * center_fade * reveal;
                    let mut strength = 0.0f32;
                    if !ripples.is_empty() {
                        let warp =
                            (value_noise3(x / 60., y / 60., t * 0.6) - 0.5) * 34.;
                        for (rx, ry, pa, pb, start) in &ripples {
                            let age =
                                now.saturating_duration_since(*start).as_millis() as f32;
                            let ddx = x - rx;
                            let ddy = y - ry;
                            let dist = (ddx * ddx + ddy * ddy).sqrt();
                            let radius = age * 0.55;
                            if (dist - radius).abs() > 2. * 30. + 34. + 0.1 * radius {
                                continue;
                            }
                            let ang = ddy.atan2(ddx);
                            let lobes = radius
                                * (0.06 * (3. * ang + pa + t).sin()
                                    + 0.04 * (5. * ang + pb - t * 1.3).sin());
                            let off = (dist - radius - lobes - warp) / 30.;
                            if off > -2. && off < 2. {
                                let fade = 1. - age / ASCII_RIPPLE_LIFE_MS as f32;
                                strength += (-off * off).exp() * fade * fade;
                            }
                        }
                        strength = strength.min(1.);
                    }
                    let use_accent = strength > 0.08;
                    // Scramble: visible cells re-roll their glyph.
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let roll = rng as f32 / 4294967296.0;
                    if (ink_alpha >= 0.01 || use_accent)
                        && roll < 0.025 + 0.35 * strength
                    {
                        self.ascii_cells[idx] =
                            (rng as usize % ASCII_GLYPHS.len()) as u8;
                    }
                    let ch = self.ascii_cells[idx] as char;
                    if use_accent {
                        acc.push(ch);
                        dim.push(' ');
                        bright.push(' ');
                    } else if ink_alpha < 0.02 {
                        dim.push(' ');
                        bright.push(' ');
                        acc.push(' ');
                    } else if ink_alpha < 0.09 {
                        dim.push(ch);
                        bright.push(' ');
                        acc.push(' ');
                    } else {
                        bright.push(ch);
                        dim.push(' ');
                        acc.push(' ');
                    }
                }
                rows_dim.push(dim);
                rows_bright.push(bright);
                rows_accent.push(acc);
            }
            let layer = |rows: Vec<String>, color: gpui::Hsla| {
                let mut col = v_flex().gap_0();
                for line in rows {
                    col = col.child(
                        div()
                            .text_size(px(10.))
                            .line_height(px(13.))
                            .whitespace_nowrap()
                            .text_color(color)
                            .child(line),
                    );
                }
                div()
                    .absolute()
                    .inset_0()
                    .overflow_hidden()
                    .child(col)
            };
            let wm = if self.show_ascii_bg {
                div()
                    .size_full()
                    .overflow_hidden()
                    .relative()
                    .child(layer(rows_dim, subtle.opacity(0.65).into()))
                    .child(layer(rows_bright, subtle.into()))
                    .child(layer(rows_accent, theme.primary.opacity(0.75).into()))
            } else {
                div().size_full()
            };
            // Clicks on empty landing space spawn accent ripples
            // (upstream AsciiWaveBackground pointer handler; interactive
            // children consume their own clicks first).
            let pane_ox = 51. + if self.sidebar_collapsed { 0. } else { 257. };
            let mut home = v_flex()
                .id("home-landing")
                .relative()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap_4()
                .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                    let fx = ev.position().x.as_f32() - pane_ox;
                    let fy = ev.position().y.as_f32() - 47.;
                    if fx < 0. || fy < 0. {
                        return;
                    }
                    let mut rng = ev.position().x.as_f32() as u32
                        ^ (ev.position().y.as_f32() as u32) << 16;
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let phase_a = (rng as f32 / 4294967296.0) * 6.2831855;
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let phase_b = (rng as f32 / 4294967296.0) * 6.2831855;
                    this.ascii_ripples.push((
                        fx,
                        fy,
                        phase_a,
                        phase_b,
                        std::time::Instant::now(),
                    ));
                    if this.ascii_ripples.len() > ASCII_RIPPLE_MAX {
                        this.ascii_ripples.remove(0);
                    }
                    cx.notify();
                }))
                .child(div().absolute().inset_0().overflow_hidden().child(wm));
            if !self.sponsor_dismissed {
                // Upstream SponsorLandingBanner: centered, max-w-520, borderless
                // row — star + title + subtitle; dismiss X appears only on hover.
                home = home.child(
                    div().absolute().top(px(15.)).left_0().w_full().child(
                        h_flex().w_full().justify_center().px_6().child(
                            h_flex()
                                .relative()
                                .id("sponsor-banner")
                                .group("sponsor-banner")
                                .cursor_pointer()
                                .w(px(520.))
                                .rounded(px(16.))
                                .px_4()
                                .py(px(12.))
                                .gap_4()
                                .items_center()
                                .hover(|d| d.bg(rgba(0x0000000a)))
                                .on_click(cx.listener(|_this, _ev, _w, cx| {
                                    cx.open_url("https://www.trysynara.com/sponsor");
                                }))
                                .child(
                                    div()
                                        .size(px(40.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg_icon(icons::STAR)
                                                .size(px(20.))
                                                .text_color(theme.primary),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .whitespace_nowrap()
                                                .child("Support Synara"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .text_color(theme.muted_foreground)
                                                .whitespace_nowrap()
                                                .child("Explore sponsorships and help fund development"),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("sponsor-dismiss")
                                        .absolute()
                                        .top(px(-6.))
                                        .right(px(-6.))
                                        .size(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.background)
                                        .cursor_pointer()
                                        .invisible()
                                        .group_hover("sponsor-banner", |d| d.visible())
                                        .child(
                                            Icon::new(AssetIcon::X)
                                                .size(px(12.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .on_click(cx.listener(|this, _ev, _w, cx| {
                                            this.sponsor_dismissed = true;
                                            cx.notify();
                                        })),
                                ),
                        ),
                    ),
                );
            }
            pane = pane.child(
                home.child(svg_icon(icons::LOGO).size(px(40.)))
                    .child(
                        div()
                            .mt(px(-17.))
                            .text_size(px(30.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color({
                                let mut c = theme.foreground;
                                c.a *= 0.95;
                                c
                            })
                            .child(landing_title.clone().unwrap_or_default()),
                    ),
            );
        }
        let thread_detail = self.detail.clone().unwrap_or(Value::Null);
        let thread_v = get(&thread_detail, "thread");
        let has_thread = self.selected_thread.is_some();
        // Upstream "Show debug task banner" flag: a local sample
        // ActiveTaskList for UI testing (1 in-progress, 1 pending, 1 done).
        if self.feature_flags[2] && has_thread {
            let sample_tasks: [(&'static str, u8); 3] = [
                ("Inspect banner layout without overlapping transcript text", 0),
                ("Confirm compact task banner width", 1),
                ("Verify sidebar task controls", 2),
            ];
            let mut banner = v_flex()
                .mx(px(16.))
                .mt(px(8.))
                .w(px(280.))
                .rounded(px(10.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .shadow_md()
                .px(px(10.))
                .py(px(8.))
                .gap(px(4.));
            for (task, status) in sample_tasks {
                let (glyph, color) = match status {
                    0 => ("\u{25cf}", theme.accent),          // inProgress
                    1 => ("\u{25cb}", theme.muted_foreground), // pending
                    _ => ("\u{2713}", theme.muted_foreground), // completed
                };
                banner = banner.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(color)
                                .child(glyph),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(if status == 2 {
                                    theme.muted_foreground
                                } else {
                                    theme.foreground
                                })
                                .child(task),
                        ),
                );
            }
            pane = pane.child(banner);
        }
        pane = pane.child(self.transcript_body(&thread_detail, has_thread, show_landing, cx));
        // ---- files browser / viewer ----
        let mut files_pane = v_flex().flex_1().h_full().p_4().gap_1();
        if self.view_files {
            if let Some((path, text)) = self.open_file.clone() {
                if self.file_editor.is_none() {
                    let ed = cx.new(|cx| EditorState::new(window, cx));
                    cx.subscribe_in(
                        &ed,
                        window,
                        |this: &mut Self, _state, event, _w, cx| {
                            if let InputEvent::Change = event {
                                this.file_dirty = true;
                                cx.notify();
                            }
                        },
                    )
                    .detach();
                    self.file_editor = Some(ed);
                }
                let editor = self.file_editor.clone().unwrap();
                let sync_key = format!(
                    "{}:{}",
                    path,
                    self.open_file_meta
                        .as_ref()
                        .map(|m| m.0.as_str())
                        .unwrap_or("")
                );
                if !self.editing_file && self.editor_synced.as_deref() != Some(&sync_key) {
                    let lang = Self::editor_language(&path);
                    let t = text.clone();
                    editor.update(cx, |s, cx| {
                        s.set_highlighter(lang, cx);
                        s.set_value(t, window, cx);
                    });
                    self.editor_synced = Some(sync_key);
                }
                let p = path.clone();
                let editing = self.editing_file;
                let mut header = h_flex()
                    .gap_2()
                    .child(
                        Button::new("close-file")
                            .ghost()
                            .label("x")
                            .on_click(cx.listener(|this, _ev, _w, cx| this.close_file(cx))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(p),
                    );
                if editing {
                    if self.file_dirty {
                        header = header.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Unsaved"),
                        );
                    }
                    header = header
                        .child(
                            Button::new("save-file")
                                .primary()
                                .label("Save")
                                .on_click(cx.listener(|this, _ev, _w, cx| this.save_file(cx))),
                        )
                        .child(
                            Button::new("cancel-edit")
                                .ghost()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.editing_file = false;
                                    this.editor_synced = None;
                                    cx.notify();
                                })),
                        );
                } else {
                    header = header.child(
                        Button::new("open-in-editor")
                            .ghost()
                            .label("Open in\u{2026}")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.open_in_menu = !this.open_in_menu;
                                cx.notify();
                            })),
                    );
                    header = header.child(
                        Button::new("edit-file")
                            .ghost()
                            .label("Edit")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.editing_file = true;
                                cx.notify();
                            })),
                    );
                }
                files_pane = files_pane
                    .child(header)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .child(Editor::new(&editor).readonly(!editing)),
                    );
                if self.open_in_menu {
                    // Upstream "Open in editor" (EditorActionPicker): launches
                    // the file in an installed external editor app.
                    const EDITOR_APPS: &[(&str, &str)] = &[
                        ("Cursor", "Cursor"),
                        ("VS Code", "Visual Studio Code"),
                        ("VSCodium", "VSCodium"),
                        ("Zed", "Zed"),
                        ("Windsurf", "Windsurf"),
                        ("Sublime Text", "Sublime Text"),
                    ];
                    let file_path = path.clone();
                    let mut menu = v_flex().gap_0p5().py_1();
                    let mut any_app = false;
                    for (label, app) in EDITOR_APPS {
                        if !std::path::Path::new(&format!(
                            "/Applications/{app}.app"
                        ))
                        .exists()
                        {
                            continue;
                        }
                        any_app = true;
                        let app_name = (*app).to_string();
                        let p = file_path.clone();
                        menu = menu.child(
                            div()
                                .id(ElementId::Name(
                                    format!("open-in-{label}").into(),
                                ))
                                .cursor_pointer()
                                .px_3()
                                .py(px(6.))
                                .rounded_md()
                                .text_size(px(13.))
                                .text_color(theme.foreground)
                                .child((*label).to_string())
                                .on_click(cx.listener(
                                    move |this, _ev, _w, cx| {
                                        this.open_in_menu = false;
                                        let app = app_name.clone();
                                        let path = p.clone();
                                        std::thread::spawn(move || {
                                            let _ = std::process::Command::new(
                                                "open",
                                            )
                                            .args(["-a", &app, &path])
                                            .spawn();
                                        });
                                        cx.notify();
                                    },
                                )),
                        );
                    }
                    if !any_app {
                        menu = menu.child(
                            div()
                                .px_3()
                                .py(px(6.))
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("No external editors found"),
                        );
                    }
                    files_pane = files_pane.child(
                        div()
                            .mx_2()
                            .mb_2()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .bg(popover_bg)
                            .child(menu),
                    );
                }
            } else {
                let parent = self
                    .files_dir
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_string())
                    .filter(|d| !d.is_empty());
                let mut header = h_flex().gap_2().child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(self.files_dir.clone()),
                );
                if let Some(parent) = parent {
                    header = header.child(
                        div()
                            .id("files-up")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.primary)
                            .child("..")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.load_dir(parent.clone(), cx)
                            })),
                    );
                }
                files_pane = files_pane.child(header);
                // Upstream WorkspaceSearchInputHeader: fuzzy file search over
                // the workspace; empty query shows the tree, non-empty the
                // flat result rows.
                files_pane = files_pane.child(
                    h_flex()
                        .items_center()
                        .gap_1p5()
                        .px_2()
                        .h(px(30.))
                        .rounded_md()
                        .bg(subtle)
                        .child(
                            svg_icon(icons::SEARCH)
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div().flex_1().child(
                                Input::new(&self.file_search).appearance(false),
                            ),
                        ),
                );
                let fs_query = self.file_search.read(cx).value().trim().to_string();
                if !fs_query.is_empty() {
                    if self.file_search_results.is_empty() {
                        files_pane = files_pane.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("loading\u{2026}"),
                        );
                    }
                    let mut srows = v_flex().gap_0p5();
                    for entry in self.file_search_results.clone() {
                        let rel = str_at(&entry, "path").to_string();
                        let name = rel
                            .rsplit('/')
                            .next()
                            .unwrap_or(rel.as_str())
                            .to_string();
                        let dir_part = rel
                            .rsplit_once('/')
                            .map(|(d, _)| d.to_string())
                            .unwrap_or_default();
                        let dir = self.files_dir.clone();
                        let abs = if rel.starts_with('/') {
                            rel.clone()
                        } else {
                            format!("{dir}/{rel}")
                        };
                        let mut e2 = entry.clone();
                        if let Some(m) = e2.as_object_mut() {
                            m.insert("path".into(), json!(abs));
                        }
                        srows = srows.child(
                            h_flex()
                                .id(ElementId::Name(
                                    format!("fs-{rel}").into(),
                                ))
                                .w_full()
                                .h(px(32.))
                                .items_center()
                                .gap_1p5()
                                .px_2()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|d| d.bg(subtle))
                                .child(
                                    svg_icon(icons::FILE_TEXT)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .whitespace_nowrap()
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(dir_part),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.file_search
                                        .update(cx, |s, cx| s.set_value("", _w, cx));
                                    this.file_search_results.clear();
                                    this.open_entry(e2.clone(), cx);
                                }))
                                .on_mouse_down(
                                    gpui::MouseButton::Right,
                                    cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                                        this.ctx_menu = None;
                                        this.file_ctx_menu = Some((
                                            abs.clone(),
                                            ev.position.x.into(),
                                            ev.position.y.into(),
                                        ));
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                ),
                        );
                    }
                    files_pane = files_pane.child(srows);
                } else {
                    if self.files_loading {
                        files_pane = files_pane.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("loading\u{2026}"),
                        );
                    }
                    // Explorer tree (upstream WorkspaceFilesTreeBody): lazy
                    // expand/collapse directories under the workspace root.
                    let mut rows: Vec<gpui::AnyElement> = Vec::new();
                    let root_entries = self.files.clone();
                    self.file_tree_rows(&root_entries, 0, &theme, subtle.into(), &mut rows, cx);
                    files_pane = files_pane.child(v_flex().gap_0p5().children(rows));
                }
            }
        }

        // ---- dev servers pane ----
        let mut dev_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_dev {
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("DEV SERVERS"),
                    )
                    .child(
                        Button::new("dev-refresh")
                            .ghost()
                            .label("Refresh")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.fetch_dev_servers(cx)
                            })),
                    ),
            );
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.dev_cmd)))
                    .child(
                        Button::new("dev-run")
                            .primary()
                            .label("Run")
                            .on_click(cx.listener(|this, _ev, w, cx| this.dev_run(w, cx))),
                    ),
            );
            if self.dev_servers.is_empty() {
                dev_pane = dev_pane.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No dev servers running"),
                );
            }
            for s in self.dev_servers.clone() {
                let pid_key = str_at(&s, "projectId").to_string();
                let status = str_at(&s, "status");
                let color = match status {
                    "running" => theme.primary,
                    "failed" | "exited" | "stopped" => theme.danger,
                    _ => theme.muted_foreground,
                };
                dev_pane = dev_pane.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .font_family("monospace")
                                .child(str_at(&s, "command").to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(color)
                                .child(format!("{status} (pid {})", get(&s, "pid"))),
                        )
                        .child(
                            Button::new(ElementId::Name(
                                format!("dev-stop-{pid_key}").into(),
                            ))
                            .ghost()
                            .label("Stop")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.dev_stop(pid_key.clone(), cx)
                            })),
                        ),
                );
            }
        }

        // ---- inbox pane (upstream /inbox recap layout) ----
        let mut inbox_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_inbox {
            use chrono::Timelike as _;
            let now = chrono::Local::now();
            let slot = match now.hour() {
                5..=11 => "Good morning",
                12..=17 => "Good afternoon",
                _ => "Good evening",
            };
            let greeting = format!("{}. {}.", slot, now.format("%A, %B %-d"));
            // Upstream InboxTaskList groups over the activity threads —
            // computed here so the greeting sub and the list share the result.
            let activity_threads: Vec<Value> =
                arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads").to_vec();
            let (needs_you, working, finished, failed) =
                inbox_groups(&activity_threads, &self.dismissed_status, &self.visited_threads);
            let needs_count = needs_you.len();
            let sub = if needs_count == 0 {
                "You're all caught up.".to_string()
            } else {
                let n = needs_count;
                format!("{n} thing{} need{} you.", if n == 1 { "" } else { "s" }, if n == 1 { "s" } else { "" })
            };
            let tile = |children: gpui::AnyElement| {
                div().rounded(px(16.)).bg(soft_bg).child(children)
            };

            let header = h_flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Inbox"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(format!("{greeting} {sub}")),
                );

            // Tasks card (left column, 4/12 of the upstream @5xl grid)
            let tasks_card = v_flex()
                .w_full()
                .gap_2()
                .p_4()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_size(px(13.)).child(format!("Today's tasks {}", self.inbox_tasks.len())))
                        .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("All tasks")),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(AssetIcon::CircleDot)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .whitespace_nowrap()
                                .text_color(theme.muted_foreground)
                                .child("Add a task for today, or press Tab to hand it to an agent"),
                        ),
                );
            // Task rows (upstream InboxTasks TaskListItem): check toggle,
            // title, due-day chip; rows linked to a thread open it on click.
            let today = now.format("%Y-%m-%d").to_string();
            let mut rows = v_flex().w_full().gap_1();
            for task in self.inbox_tasks.clone() {
                let title = str_at(&task, "title").to_string();
                if title.is_empty() {
                    continue;
                }
                let id = str_at(&task, "id")
                    .to_string()
                    .replace("null", "");
                let done = !str_at(&task, "completedAt").is_empty()
                    && str_at(&task, "completedAt") != "null";
                let due = str_at(&task, "dueDate").to_string();
                let due_label = if due.is_empty() || due == "null" {
                    String::new()
                } else if due < today {
                    "Overdue".to_string()
                } else if due == today {
                    "Today".to_string()
                } else {
                    due[..due.len().min(10)].to_string()
                };
                let tid = str_at(&task, "threadId").to_string();
                let mut row = h_flex()
                    .id(ElementId::Name(format!("inbox-task-{id}").into()))
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py(px(6.))
                    .rounded_md()
                    .child(
                        div()
                            .id(ElementId::Name(
                                format!("inbox-task-check-{id}").into(),
                            ))
                            .cursor_pointer()
                            .child(
                                Icon::new(if done {
                                    AssetIcon::CircleCheck
                                } else {
                                    AssetIcon::CircleDot
                                })
                                .size(px(14.))
                                .text_color(if done {
                                    theme.success
                                } else {
                                    theme.muted_foreground
                                }),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.toggle_todo(id.clone(), !done, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .text_color(if done {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            })
                            .child(title),
                    );
                if !due_label.is_empty() {
                    row = row.child(
                        div()
                            .rounded_full()
                            .px(px(6.))
                            .py(px(1.))
                            .bg(theme.muted)
                            .text_size(px(11.))
                            .text_color(if due_label == "Overdue" {
                                theme.danger
                            } else {
                                theme.muted_foreground
                            })
                            .child(due_label),
                    );
                }
                if !tid.is_empty() && tid != "null" {
                    row = row.cursor_pointer().on_click(cx.listener(
                        move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                    ));
                }
                rows = rows.child(row);
            }
            let tasks_card = tasks_card.child(rows);

            // Right column: recap + usage tiles
            let mut right_col = v_flex().flex_1().gap_3();
            let recap_text = self
                .inbox_recap
                .as_ref()
                .map(|r| str_at(r, "text").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "Nothing ran yet today. This fills in as your agents work.".to_string());
            right_col = right_col.child(tile(
                div().p_4().text_size(px(13.)).child(recap_text).into_any_element(),
            ));
            let provider = self
                .inbox_usage
                .first()
                .map(|u| str_at(u, "provider").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "Devin".to_string());
            let pct = self
                .inbox_usage
                .first()
                .and_then(|u| get(u, "usedPct").as_f64().or_else(|| get(u, "remainingPct").as_f64()))
                .map(|p| if get(self.inbox_usage.first().unwrap_or(&Value::Null), "usedPct").is_f64() { 100.0 - p } else { p })
                .unwrap_or(93.0);
            let resets = self
                .inbox_usage
                .first()
                .map(|u| str_at(u, "resetsIn").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "5d 18h".to_string());
            right_col = right_col.child(tile(
                v_flex()
                    .p_4()
                    .gap_2()
                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(format!("Left on {provider}, weekly")))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .child(svg_icon(icons::CODEX).size(px(18.)).text_color(theme.foreground))
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("{pct:.0}%")),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(48.))
                                    .h(px(6.))
                                    .rounded(px(3.))
                                    .bg(theme.border)
                                    .child(div().w(gpui::relative((pct / 100.0) as f32)).h_full().rounded(px(3.)).bg(rgb(0x22a865))),
                            )
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(format!("Resets in {resets}"))),
                    )
                    .into_any_element(),
            ));

            let grid = h_flex()
                .gap(px(16.))
                .items_start()
                .child(
                    tile(tasks_card.into_any_element())
                        .w(gpui::relative(4.0 / 12.0)),
                )
                .child(right_col);

            let mut page = v_flex()
                .flex_1()
                .px(px(32.))
                .pt(px(74.))
                .pb(px(32.))
                .gap_4()
                .child(header)
                .child(grid);

            // Upstream InboxTaskList groups (computed above for the greeting).
            if !(needs_you.is_empty()
                && working.is_empty()
                && finished.is_empty()
                && failed.is_empty())
            {
                let reason = |k: &str| match k {
                    "approval" => "approve a command",
                    "input" => "has a question",
                    _ => "plan ready",
                };
                let mut specs: Vec<(&str, AssetIcon, Hsla, Vec<(Value, String)>)> = Vec::new();
                if !needs_you.is_empty() {
                    specs.push((
                        "Needs you",
                        AssetIcon::TriangleAlert,
                        theme.warning,
                        needs_you
                            .iter()
                            .map(|(k, t)| (t.clone(), reason(k).to_string()))
                            .collect(),
                    ));
                }
                if !working.is_empty() {
                    specs.push((
                        "In progress",
                        AssetIcon::CircleDot,
                        theme.accent,
                        working.iter().map(|t| (t.clone(), "working".to_string())).collect(),
                    ));
                }
                if !finished.is_empty() {
                    specs.push((
                        "Ready for review",
                        AssetIcon::CircleCheck,
                        theme.success,
                        finished.iter().map(|t| (t.clone(), "finished".to_string())).collect(),
                    ));
                }
                if !failed.is_empty() {
                    specs.push((
                        "Failed",
                        AssetIcon::X,
                        theme.danger,
                        failed.iter().map(|t| (t.clone(), "failed".to_string())).collect(),
                    ));
                }
                let mut tcard = v_flex().w_full().p_4().gap_3();
                for (label, ic, color, group_rows) in specs {
                    let mut sec = v_flex().w_full().gap_1().child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(format!("{label} {}", group_rows.len())),
                    );
                    for (t, note) in group_rows {
                        let tid = str_at(&t, "id").to_string();
                        let title = str_at(&t, "title").to_string();
                        sec = sec.child(
                            h_flex()
                                .id(ElementId::Name(format!("inbox-thr-{tid}").into()))
                                .w_full()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py(px(6.))
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.muted))
                                .child(Icon::new(ic).size(px(14.)).text_color(color))
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(13.))
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(note),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.open_thread(tid.clone(), cx);
                                })),
                        );
                    }
                    tcard = tcard.child(sec);
                }
                page = page.child(tile(tcard.into_any_element()));
            }
            if self.inbox_signed_in != Some(true) {
                page = page.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Sign in to automatically save private recaps across your devices."),
                        )
                        .child(Button::new("inbox-signin").outline().label("Sign in")),
                );
            }
            inbox_pane = inbox_pane.child(page);
        }

        // Provider-update toast (upstream toastManager warning card): global
        // overlay over the main column, centered, does not shift content.
        let mut update_toast = div().absolute().top(px(-12.)).w_full().flex().justify_center();
        let mut toast_shown = false;
        if !self.inbox_updates.is_empty() {
            toast_shown = true;
            let n = self.inbox_updates.len();
            let pname = {
                let id = str_at(&self.inbox_updates[0], "provider");
                match id {
                    "codex" => "Codex", "claudeAgent" | "claude" => "Claude",
                    "cursor" => "Cursor", "antigravity" => "Antigravity",
                    "grok" => "Grok", "droid" => "Droid", "devin" => "Devin",
                    "opencode" => "OpenCode", "pi" => "Pi",
                    "omp" => "Oh My Pi", other => other,
                }
            };
            let (title, desc) = if n == 1 {
                (
                    format!("{pname} update available"),
                    format!("{pname} has a newer version available."),
                )
            } else {
                (
                    format!("{n} provider updates available"),
                    format!(
                        "{pname} and {} more provider{} have newer versions available.",
                        n - 1,
                        if n == 2 { "" } else { "s" }
                    ),
                )
            };
            update_toast = update_toast.child(
                h_flex()
                    .w(px(364.))
                    .gap_2()
                    .items_start()
                    .rounded(px(12.))
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .p(px(14.))
                    .child(
                        Icon::new(AssetIcon::TriangleAlert)
                            .size(px(16.))
                            .text_color(rgb(0xc24904)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(desc),
                            )
                            .child(
                                h_flex()
                                    .mt(px(6.))
                                    .gap(px(16.))
                                    .child(div().text_size(px(13.)).child("Review updates"))
                                    .child(div().text_size(px(13.)).child("Update all")),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("\u{00d7}"),
                    ),
            );
        }

        // ---- composer ----
        // chat-composer-shell squircle: 19px radius card, hairline border,
        // soft surface shadow (approximated by the border + card tone).
        let mode_accent = rgb(0xc24904);
        let draft_text = self.composer.read(cx).value().to_string();
        let draft_empty = draft_text.trim().is_empty();
        // Upstream ComposerPromptEditor: font-system-ui text-chat (13px)
        // leading-relaxed (1.625 -> ~21.1px/line), min-height 2lh (~42px),
        // padding x12/top12/bottom8; placeholder sits at the text origin.
        let mut input_zone = div().relative().child(
            div()
                .min_h(px(if self.selected_thread.is_some() {
                    24.
                } else {
                    42.25
                }))
                .child(Input::new(&self.composer).appearance(false)),
        );
        if draft_empty {
            // Placeholder rendered on top: the web uses muted-foreground/40,
            // much lighter than the editor's own muted foreground tone.
            input_zone = input_zone.child(
                div()
                    .absolute()
                    .top(px(if self.selected_thread.is_some() {
                        1.
                    } else {
                        12.
                    }))
                    .left(px(12.))
                    .text_size(px(13.))
                    .line_height(px(21.125))
                    .text_color(rgba(0x67676766))
                    .child("Do anything in Synara"),
            );
        }
        let in_thread = self.selected_thread.is_some();
        // "+" extras trigger: 28px filled circle, black/3%; opens the upstream
        // "Add" extras menu (files, attach window, goal, plan/debug/fast).
        let extras_weak = cx.weak_entity();
        let extras_cell = self.client.clone();
        let imode = self.interaction_mode;
        let fast_on = self.fast_mode;
        let plus_btn = Button::new("composer-attach")
            .compact()
            .icon(svg_icon(icons::PLUS).size(px(14.)))
            .rounded(ButtonRounded::Size(px(14.)))
            .with_variant(ButtonVariant::Custom(
                ButtonCustomVariant::new(cx)
                    .color(chip_bg.into())
                    .foreground(theme.foreground)
                    .hover(subtle.into()),
            ))
            .w(px(28.))
            .h(px(28.))
            .dropdown_menu(move |menu, window, cx| {
                let wins: Vec<(String, String, String)> = ensure_client(&extras_cell)
                    .and_then(|c| c.call("computer.listWindows", json!({})))
                    .and_then(|v| v.get("windows").cloned())
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|w| {
                        let id = w.get("id").and_then(|s| s.as_str())?.to_string();
                        let title = w
                            .get("title")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let app = w
                            .get("appName")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some((id, title, app))
                    })
                    .take(12)
                    .collect();
                let weak = extras_weak.clone();
                let mut m = menu
                    .min_w(px(280.))
                    .item(PopupMenuItem::label("Add"))
                    .item(
                        PopupMenuItem::new("Files and folders")
                            .icon(svg_icon(icons::PAPERCLIP).size(px(14.)))
                            .on_click(move |_, _w, cx| {
                                let _ = weak.update(cx, |this, cx| this.attach(cx));
                            }),
                    );
                let weak = extras_weak.clone();
                m = m.submenu_with_icon(
                    Some(svg_icon(icons::WINDOW).size(px(14.))),
                    "Attach window",
                    window,
                    cx,
                    move |sm, _, _| {
                        let mut sm = sm.min_w(px(260.));
                        if wins.is_empty() {
                            sm = sm.item(PopupMenuItem::new("No windows").disabled(true));
                        } else {
                            for (id, title, app) in &wins {
                                let label = if title.is_empty() {
                                    app.clone()
                                } else {
                                    format!("{app} — {title}")
                                };
                                let id = id.clone();
                                let title = title.clone();
                                let weak = weak.clone();
                                sm = sm.item(
                                    PopupMenuItem::new(label)
                                        .icon(svg_icon(icons::WINDOW).size(px(14.)))
                                        .on_click(move |_, _w, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.capture_window(id.clone(), title.clone(), cx);
                                            });
                                        }),
                                );
                            }
                        }
                        sm
                    },
                );
                let weak = extras_weak.clone();
                m = m.item(
                    PopupMenuItem::new("Goal")
                        .icon(svg_icon(icons::GOAL).size(px(14.)))
                        .on_click(move |_, w, cx| {
                            let _ = weak.update(cx, |this, cx| this.insert_goal(w, cx));
                        }),
                );
                for (label, icon, mode) in [
                    ("Plan Mode", icons::TODOS, "plan"),
                    ("Debug Mode", icons::BUG, "debug"),
                ] {
                    let weak = extras_weak.clone();
                    m = m.item(
                        PopupMenuItem::new(label)
                            .icon(svg_icon(icon).size(px(14.)))
                            .checked(imode == mode)
                            .on_click(move |_, _w, cx| {
                                let _ = weak.update(cx, |this, cx| {
                                    this.interaction_mode = if this.interaction_mode == mode {
                                        "default"
                                    } else {
                                        mode
                                    };
                                    cx.notify();
                                });
                            }),
                    );
                }
                let weak = extras_weak.clone();
                m.item(
                    PopupMenuItem::new("Fast Mode")
                        .icon(svg_icon(icons::ZAP).size(px(14.)))
                        .checked(fast_on)
                        .on_click(move |_, _w, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.fast_mode = !this.fast_mode;
                                cx.notify();
                            });
                        }),
                )
            });
        // Upstream runtime-mode chip opens a picker menu: label + description
        // rows ("Ask for approval" / "Approve for me" / "Full access") with a
        // check on the active mode.
        let mode_weak = cx.weak_entity();
        let cur_mode = self.runtime_mode;
        let build_mode_chip = move |cx: &mut Context<Self>| {
            let weak = mode_weak.clone();
            let active = cur_mode;
            Button::new("composer-mode")
                .compact()
                .icon(
                    svg_icon(match active {
                        "auto" => icons::SHIELD_CODE,
                        _ => icons::SHIELD_ACCESS,
                    })
                    .size(px(14.)),
                )
                .label(match active {
                    "approval-required" => "Ask for approval",
                    "auto" => "Approve for me",
                    _ => "Full access",
                })
                .dropdown_caret(true)
                .with_variant(ButtonVariant::Custom(
                    ButtonCustomVariant::new(cx)
                        .color(rgba(0x00000000).into())
                        .foreground(mode_accent.into())
                        .hover(rgba(0xc2490414).into()),
                ))
                .dropdown_menu(move |menu, _w, _cx| {
                    const MODES: [(&str, &str, &str); 3] = [
                        (
                            "approval-required",
                            "Ask for approval",
                            "Always ask to edit external files and use the internet",
                        ),
                        (
                            "auto",
                            "Approve for me",
                            "Only ask for actions detected as potentially unsafe",
                        ),
                        (
                            "full-access",
                            "Full access",
                            "Unrestricted access to the internet and any file on your computer",
                        ),
                    ];
                    let mut m = menu.min_w(px(300.));
                    for (id, label, desc) in MODES {
                        let checked = active == id;
                        let weak2 = weak.clone();
                        m = m.item(
                            PopupMenuItem::element(move |_, _| {
                                v_flex()
                                    .gap_0p5()
                                    .child(div().text_size(px(13.)).child(label))
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgba(0x777777aa))
                                            .child(desc),
                                    )
                            })
                            .checked(checked)
                            .on_click(move |_, _w, cx| {
                                let _ = weak2.update(cx, |this, cx| {
                                    this.runtime_mode = id;
                                    cx.notify();
                                });
                            }),
                        );
                    }
                    m
                })
        };
        // The thread composer labels the chip with the thread's own model.
        let thread_model = str_at(get(thread_v, "modelSelection"), "label")
            .to_string();
        let thread_model = if thread_model.is_empty() {
            str_at(get(thread_v, "modelSelection"), "model").to_string()
        } else {
            thread_model
        };
        let model_label = if in_thread {
            if thread_model.is_empty() {
                // Upstream seeds temporary chat threads with Devin Small.
                "Devin Small".to_string()
            } else {
                thread_model
            }
        } else {
            "GPT-6 Astra".to_string()
        };
        let build_model_chip = move || {
            Button::new("composer-model")
                .ghost()
                .compact()
                .icon(svg_icon(icons::CODEX))
                .label(model_label.clone())
                .dropdown_caret(true)
                .dropdown_menu(|menu, _, _| {
                    menu.item(PopupMenuItem::new("\u{2713} GPT-6 Astra (codex)"))
                })
        };
        // While the selected thread's turn is live, upstream swaps the send
        // button for a prominent dark stop square (thread.turn.interrupt).
        let turn_live = in_thread
            && matches!(session_phase(self.detail.as_ref()), "running" | "connecting");
        let send_btn = if turn_live {
            div()
                .id("send")
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme.foreground)
                .cursor_pointer()
                .child(div().size(px(10.)).rounded(px(2.)).bg(theme.background))
                .on_click(cx.listener(|this, _ev, _w, cx| this.stop(cx)))
                .into_any_element()
        } else {
            // Empty draft renders the muted (disabled) circle like
            // the web composer; filled primary once there's text.
            div()
                .id("send")
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(if draft_empty {
                    theme.foreground.opacity(0.2)
                } else {
                    theme.foreground
                })
                .cursor_pointer()
                .child(
                    svg_icon(icons::SEND)
                        .size(px(20.))
                        .text_color(theme.background),
                )
                .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx)))
                .into_any_element()
        };
        // Upstream: started threads use the single-row composer while the draft
        // is one plain line — the card holds [+ | input | send] and the pickers
        // move to a row under the card. Multi-line drafts and the home composer
        // keep the block layout with controls inside.
        let row_composer = in_thread && !draft_text.contains('\n');
        let composer_border = if self.composer_thread_over {
            theme.accent
        } else {
            theme.border
        };
        let composer_card: Div = if row_composer {
            div()
                .w_full()
                .max_w(px(755.))
                .border_1()
                .border_color(composer_border)
                .rounded(px(19.))
                .bg(sidebar_bg)
                .shadow_md()
                .px_2()
                .py(px(4.))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(plus_btn)
                        .child(div().flex_1().min_w_0().child(input_zone))
                        .child(send_btn),
                )
        } else {
            v_flex()
                .w_full()
                .max_w(px(755.))
                .border_1()
                .border_color(composer_border)
                .rounded(px(19.))
                .bg(sidebar_bg)
                .shadow_md()
                .px_2()
                .pt(px(4.))
                .pb(px(8.))
                .gap(px(8.))
                .child(input_zone)
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(plus_btn)
                        .child(build_mode_chip(cx))
                        .child(div().flex_1())
                        .child(build_model_chip())
                        .child(send_btn),
                )
        };
        let chat_mw = self.chat_max_w();
        let mut composer_col = v_flex()
            .w_full().max_w(chat_mw)
            .gap_0()
            // Thread rows drop here as an @mention (upstream
            // useComposerThreadMentionDrop); the current thread can't
            // mention itself.
            .can_drop(|v, _w, _cx| v.is::<ThreadDrag>() || v.is::<FileDrag>())
            .on_drag_move::<ThreadDrag>(cx.listener(
                |this, _ev: &gpui::DragMoveEvent<ThreadDrag>, _w, cx| {
                    if !this.composer_thread_over {
                        this.composer_thread_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drag_move::<FileDrag>(cx.listener(
                |this, _ev: &gpui::DragMoveEvent<FileDrag>, _w, cx| {
                    if !this.composer_thread_over {
                        this.composer_thread_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drop::<FileDrag>(cx.listener(|this, drag: &FileDrag, w, cx| {
                this.composer_thread_over = false;
                let name = drag
                    .0
                    .rsplit('/')
                    .next()
                    .unwrap_or(drag.0.as_str())
                    .to_string();
                let draft = this.composer.read(cx).value().to_string();
                let sep = if draft.is_empty() || draft.ends_with(' ') { "" } else { " " };
                this.set_draft(&format!("{draft}{sep}@{name} "), w, cx);
                cx.notify();
            }))
            .on_drop::<ThreadDrag>(cx.listener(
                |this, drag: &ThreadDrag, w, cx| {
                    this.composer_thread_over = false;
                    if this.selected_thread.as_deref() == Some(drag.0.as_str()) {
                        cx.notify();
                        return;
                    }
                    let title = arr(this.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                        .iter()
                        .find(|t| str_at(t, "id") == drag.0)
                        .map(|t| str_at(t, "title").to_string())
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| drag.0.chars().take(8).collect());
                    let draft = this.composer.read(cx).value().to_string();
                    let sep = if draft.is_empty() || draft.ends_with(' ') { "" } else { " " };
                    this.set_draft(&format!("{draft}{sep}@{title} "), w, cx);
                    cx.notify();
                },
            ));
        if !self.pending_attachments.is_empty() || self.uploading {
            let mut chips = h_flex().gap_1().flex_wrap().pb_1();
            if self.uploading {
                chips = chips.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("uploading\u{2026}"),
                );
            }
            for (i, a) in self.pending_attachments.iter().enumerate() {
                let label = str_at(a, "name");
                let label = if label.is_empty() {
                    str_at(a, "fileName").to_string()
                } else {
                    label.to_string()
                };
                chips = chips.child(
                    h_flex()
                        .id(ElementId::Name(format!("att-{i}").into()))
                        .cursor_pointer()
                        .gap_1()
                        .items_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .px_2()
                        .py_0p5()
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            if i < this.pending_attachments.len() {
                                this.pending_attachments.remove(i);
                            }
                            cx.notify();
                        }))
                        .child(div().text_xs().child(format!("{label} \u{00d7}"))),
                );
            }
            composer_col = composer_col.child(chips);
        }
        // Upstream floats context chips above the card only for threads bound
        // to a worktree (envMode != "local"): project chip left, env chip right.
        let wt_path = str_at(thread_v, "worktreePath").to_string();
        let env_mode = str_at(thread_v, "envMode").to_string();
        let has_env_chips = in_thread && (!wt_path.is_empty() || (env_mode != "local" && !env_mode.is_empty()));
        if has_env_chips {
            let thread_pid = str_at(thread_v, "projectId").to_string();
            let proj_name = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .find(|p| str_at(p, "id") == thread_pid)
                .map(|p| str_at(p, "title").to_string())
                .filter(|n: &String| !n.is_empty())
                .unwrap_or_else(|| thread_pid.clone());
            let chip = |text: String| {
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child(text)
            };
            composer_col = composer_col.child(
                h_flex()
                    .w_full()
                    .items_center()
                    .pb(px(4.))
                    .child(chip(format!("\u{29c9} {proj_name}")))
                    .child(div().flex_1())
                    .child(chip(format!("\u{25cc} {}", if wt_path.is_empty() { "Temporary" } else { "Worktree" }))),
            );
        }
        if draft_text.starts_with('/') && !draft_text.contains(' ') {
            // Upstream ComposerCommandMenu: popup card above the composer
            // listing built-in slash commands filtered by the "/query".
            const SLASH_COMMANDS: [(&str, &str, &str); 18] = [
                ("clear", "Clear", "Start a fresh thread and clear the current conversation context"),
                ("compact", "Compact Context", "Compact the current thread context to free space"),
                ("model", "Model", "Switch response model for this thread"),
                ("plan", "Plan Mode", "Switch this thread into plan mode"),
                ("debug", "Debug Mode", "Switch this thread into evidence-first debug mode"),
                ("default", "Default Mode", "Switch this thread back to normal chat mode"),
                ("review", "Code Review", "Start a code review for current changes"),
                ("fork", "Fork", "Fork this thread into local or a new worktree"),
                ("side", "Sidechat", "Open a guarded Side from this thread, optionally on another provider"),
                ("status", "Status", "Show context usage and rate-limit status"),
                ("subagents", "Subagents", "Insert a prompt that asks the assistant to delegate work"),
                ("computer-use", "Computer Use", "Use Synara Computer for this request only"),
                ("fast", "Fast Mode", "Turn fast mode on or off for this thread"),
                ("export", "Export", "Download this thread as a ZIP archive (thread.json + transcript.md)"),
                ("goal", "Goal", "Set, edit, pause, resume, or clear this thread's persistent goal"),
                ("rename", "Rename", "Regenerate this thread title, or set an exact title"),
                ("feedback", "Feedback on Synara", "Send feedback to the Synara team"),
                ("automation", "Automation", "Create an automation from this thread"),
            ];
            let query = draft_text[1..].to_lowercase();
            let mut menu_card = v_flex()
                .w_full()
                .max_h(px(300.))
                .overflow_y_scrollbar()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .shadow_md()
                .py_1()
                .mb(px(6.));
            let mut shown = 0;
            for (cmd, title, desc) in SLASH_COMMANDS {
                if !query.is_empty() && !cmd.contains(query.as_str()) {
                    continue;
                }
                shown += 1;
                menu_card = menu_card.child(
                    h_flex()
                        .id(ElementId::Name(format!("slash-{cmd}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .cursor_pointer()
                        .rounded_md()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            // Commands the shell executes natively; the rest
                            // (provider/session-bound) insert the literal
                            // "/cmd " into the draft, like upstream does for
                            // provider-native commands.
                            match cmd {
                                "clear" => this.new_thread(cx),
                                "plan" | "debug" | "default" => {
                                    this.interaction_mode = cmd;
                                    this.set_draft("", w, cx);
                                    if let Some(tid) = this.selected_thread.clone() {
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.interaction-mode.set",
                                                "commandId": uuid(),
                                                "threadId": tid,
                                                "interactionMode": cmd,
                                                "createdAt": now_iso(),
                                            }),
                                            cx,
                                        );
                                    }
                                }
                                "fast" => {
                                    this.fast_mode = !this.fast_mode;
                                    this.set_draft("", w, cx);
                                }
                                "goal" => this.insert_goal(w, cx),
                                "rename" => this.start_rename(w, cx),
                                "feedback" => {
                                    this.set_draft("", w, cx);
                                    this.open_feedback(w, cx);
                                }
                                _ => this.set_draft(&format!("/{cmd} "), w, cx),
                            }
                            cx.notify();
                        }))
                        .child(
                            v_flex()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(desc),
                                ),
                        )
                        .child(
                            div()
                                .ml_2()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(format!("/{cmd}")),
                        ),
                );
            }
            if shown == 0 {
                menu_card = menu_card.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching command."),
                );
            }
            composer_col = composer_col.child(menu_card);
        } else if let Some(query) = mention_query(&draft_text) {
            // Upstream local-folder mention: "@" browses the filesystem and
            // inserts a @path token. Path-like queries list their dirname
            // and filter on the basename; plain queries list $HOME and
            // filter by substring. Directories descend (append "/").
            let q_lower = query
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_lowercase();
            let mut menu_card = v_flex()
                .w_full()
                .max_h(px(300.))
                .overflow_y_scrollbar()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .shadow_md()
                .py_1()
                .mb(px(6.));
            let mut shown = 0;
            for entry in self.mention_entries.iter() {
                let name = str_at(entry, "name").to_string();
                if name.starts_with('.')
                    || (!q_lower.is_empty() && !name.to_lowercase().contains(&q_lower))
                {
                    continue;
                }
                shown += 1;
                let is_dir = str_at(entry, "kind") == "directory";
                let path = str_at(entry, "path").to_string();
                let entry = entry.clone();
                menu_card = menu_card.child(
                    h_flex()
                        .id(ElementId::Name(format!("mention-{path}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .gap_1p5()
                        .cursor_pointer()
                        .rounded_md()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            let draft = this.composer.read(cx).value().to_string();
                            let at = draft.rfind('@').unwrap_or(draft.len());
                            let prefix = draft[..at].to_string();
                            if str_at(&entry, "kind") == "directory" {
                                let p = str_at(&entry, "path").to_string();
                                this.set_draft(&format!("{prefix}@{p}/"), w, cx);
                                this.load_mention_dir(p, cx);
                            } else {
                                let p = str_at(&entry, "path").to_string();
                                this.set_draft(&format!("{prefix}@{p} "), w, cx);
                            }
                            cx.notify();
                        }))
                        .child(
                            svg_icon(if is_dir { icons::FOLDER } else { icons::FILE_TEXT })
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(name),
                        ),
                );
                if shown >= 50 {
                    break;
                }
            }
            if shown == 0 {
                menu_card = menu_card.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching files."),
                );
            }
            composer_col = composer_col.child(menu_card);
        }
        if !self.queued_turns.is_empty() {
            // Upstream ComposerQueuedHeader: stacked panel above the composer,
            // one row per queued follow-up (steer icon + preview + actions).
            let mut panel = v_flex()
                .w_full()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .mb(px(6.))
                .py_1();
            if self.queue_paused {
                panel = panel.child(
                    h_flex()
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("Queue paused — queued follow-ups are held."),
                        )
                        .child(
                            div()
                                .id("queue-resume")
                                .cursor_pointer()
                                .px_2()
                                .py(px(2.))
                                .rounded_md()
                                .text_size(px(12.))
                                .hover(|d| d.bg(theme.secondary))
                                .child("Resume")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.queue_paused = false;
                                    cx.notify();
                                })),
                        ),
                );
            }
            for (i, qtext) in self.queued_turns.iter().enumerate() {
                let preview: String = qtext.chars().take(120).collect();
                panel = panel.child(
                    h_flex()
                        .id(ElementId::Name(format!("queued-{i}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            // Upstream Edit: move the queued text back into the
                            // composer draft.
                            if i < this.queued_turns.len() {
                                let text = this.queued_turns.remove(i);
                                this.set_draft(&text, w, cx);
                            }
                        }))
                        .child(
                            svg_icon(icons::SEND)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.))
                                .overflow_hidden()
                                .child(preview),
                        )
                        .child(
                            div()
                                .id(ElementId::Name(format!("queued-rm-{i}").into()))
                                .flex_none()
                                .cursor_pointer()
                                .px_1()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("\u{00d7}")
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    cx.stop_propagation();
                                    if i < this.queued_turns.len() {
                                        this.queued_turns.remove(i);
                                    }
                                    cx.notify();
                                })),
                        ),
                );
            }
            composer_col = composer_col.child(panel);
        }
        composer_col = composer_col.child(composer_card);
        if row_composer {
            // Pickers row under the composer card (upstream "row" layout):
            // interaction mode on the left, model picker on the right.
            composer_col = composer_col.child(
                h_flex()
                    .w_full()
                    .pt(px(8.))
                    .px(px(4.))
                    .items_center()
                    .child(build_mode_chip(cx))
                    .child(div().flex_1())
                    .child(build_model_chip()),
            );
        }
        let composer_bar = div()
            .w_full()
            .pb(px(18.))
            .child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .pr(px(4.))
                    .child(composer_col),
            );

        // ---- split pane (chat.split, cmd-\): right column with its own
        // header, transcript surface, thread picker and composer ----
        let mut split_pane = v_flex().flex_1().h_full().gap_2().p_4();
        let mut split_bar = v_flex().w_full();
        if self.split_open {
            let st_detail = self.split_detail.clone().unwrap_or(Value::Null);
            let pane_focused = self.focused_pane == 1;
            let mut header = h_flex()
                .w_full()
                .h(px(32.))
                .px_2()
                .items_center()
                .gap_2()
                .border_b_1()
                .border_color(if pane_focused {
                    theme.primary
                } else {
                    theme.border
                });
            if self.split_thread.is_some() {
                let title = str_at(get(&st_detail, "thread"), "title").to_string();
                let provider = str_at(get(&st_detail, "session"), "providerName").to_string();
                let focused = self.focused_pane == 1;
                header = header
                    .child(
                        svg_icon(provider_icon(&provider))
                            .size(px(13.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if focused { theme.foreground } else { theme.muted_foreground })
                            .child(title),
                    )
                    .child(
                        div()
                            .id("split-close")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("\u{2715}")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.split_thread = None;
                                this.split_detail = None;
                                cx.notify();
                            })),
                    );
            }
            split_pane = split_pane.child(header);
            if self.split_thread.is_some() {
                split_pane = split_pane.child(self.transcript_body(&st_detail, true, false, cx));
            } else {
                // Empty leaf: upstream SplitPaneEmptyState — centered label
                // plus a scrollable list of threads not already shown.
                split_pane = split_pane.child(
                    v_flex()
                        .w_full()
                        .pt(px(64.))
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(theme.foreground)
                                .child("Select a chat"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Pick a thread to open in this pane"),
                        ),
                );
                let mut list = v_flex().w_full().max_w(px(384.)).gap_y(px(1.));
                for t in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads") {
                    let tid = str_at(t, "id").to_string();
                    if tid.is_empty() {
                        continue;
                    }
                    let in_use = self.open_threads.contains(&tid)
                        || self.split_thread.as_deref() == Some(tid.as_str());
                    let title = str_at(t, "title").to_string();
                    let provider = str_at(get(t, "session"), "providerName").to_string();
                    let mut row = h_flex()
                        .id(SharedString::from(format!("splitpick-{tid}")))
                        .w_full()
                        .px_2()
                        .py_1p5()
                        .rounded_md()
                        .gap_2()
                        .items_center()
                        .child(
                            svg_icon(provider_icon(&provider))
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(div().flex_1().text_xs().child(title));
                    if in_use {
                        row = row.opacity(0.35);
                    } else {
                        row = row
                            .cursor_pointer()
                            .hover(|s| s.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_split_thread(tid.clone(), cx);
                            }));
                    }
                    list = list.child(row);
                }
                split_pane = split_pane.child(
                    div().flex_1().overflow_y_scrollbar().child(
                        h_flex().w_full().justify_center().child(list),
                    ),
                );
            }
            // Dropping a dragged sidebar thread assigns it to this pane
            // (upstream createFromDrop).
            split_pane = split_pane
                .can_drop(|v, _w, _cx| {
                    v.is::<ThreadDrag>() || v.is::<TabDrag>()
                })
                .on_drop::<ThreadDrag>(cx.listener(|this, drag: &ThreadDrag, _w, cx| {
                    this.open_split_thread(drag.0.clone(), cx);
                }))
                .on_drop::<TabDrag>(cx.listener(|this, drag: &TabDrag, _w, cx| {
                    this.open_split_thread(drag.0.clone(), cx);
                }));
            if let Some(input) = self.split_composer.clone() {
                let card = div()
                    .w_full()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .px_3()
                    .py_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Input::new(&input).appearance(false).w_full())
                            .child(
                                div()
                                    .id("split-send")
                                    .cursor_pointer()
                                    .child(
                                        svg_icon(icons::SEND)
                                            .size(px(14.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .on_click(cx.listener(|this, _ev, w, cx| {
                                        this.send_split(w, cx);
                                    })),
                            ),
                    );
                let chat_mw = self.chat_max_w();
                split_bar = split_bar.child(
                    div().w_full().pb(px(18.)).child(
                        h_flex().w_full().justify_center().pr(px(4.)).child(
                            v_flex().w_full().max_w(chat_mw).gap_0().child(card),
                        ),
                    ),
                );
            }
        }

        // ---- settings pane ----
        // ---- settings pane (upstream /settings layout) ----
        // Nav: (group label, [(section id, label, icon, badge)])
        let settings_nav: &[(&str, &[(&str, &str, &[u8], Option<&str>)])] = &[
            ("", &[
                ("profile", "Profile", icons::USER, None),
                ("accounts", "Accounts", icons::S_PEOPLE_ID_CARD, None),
            ]),
            ("Synara", &[
                ("general", "General", icons::S_GEAR, None),
                ("appearance", "Appearance", icons::S_COLOR_PALETTE, None),
                ("layout", "Layout", icons::S_LAYOUT_SIDEBAR, None),
                ("chats", "Chats", icons::S_CHAT_BUBBLES, None),
                ("notifications", "Notifications", icons::S_BELL, None),
                ("shortcuts", "Keyboard shortcuts", icons::S_SHORTCUT, None),
            ]),
            ("Agents", &[
                ("providers", "Providers", icons::S_PUZZLE, None),
                ("models", "Models", icons::S_BRAIN, None),
                ("skills", "Skills", icons::S_BUILDING_BLOCKS, None),
                ("usage", "Usage", icons::S_GAUGE, None),
            ]),
            ("Connections", &[
                ("connectors", "Connectors", icons::S_CONNECTORS, None),
                ("integrations", "External agents", icons::S_PLUGIN, None),
                ("devices", "Devices", icons::S_GLOBE, None),
                ("computer", "Computer use", icons::S_COMPUTER_USE, Some("Beta")),
                ("appsnap", "AppSnap", icons::S_SCREEN_CAPTURE, None),
            ]),
            ("Workspace", &[
                ("worktrees", "Worktrees", icons::S_BRANCH_SIMPLE, None),
                ("archive", "Archive", icons::S_ARCHIVE, None),
            ]),
            ("System", &[
                ("privacy", "Privacy", icons::S_SHIELD, None),
                ("advanced", "Advanced", icons::S_TOOLBOX, None),
                ("about", "About", icons::S_CIRCLE_INFO, None),
            ]),
        ];
        let section_title: &str = settings_nav
            .iter()
            .flat_map(|(_, items)| items.iter())
            .find(|(id, ..)| *id == self.settings_section)
            .map(|(_, l, ..)| *l)
            .unwrap_or("General");

        // Settings card: rounded-xl, faint fill (foreground/2%), hairline border.
        let card = |rows: gpui::AnyElement| {
            div()
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(popover_bg)
                .child(rows)
        };
        // One row: title + description left, control right.
        let dscale = self.density_scale();
        let srow = |title: &'static str, desc: &'static str, ctrl: gpui::AnyElement| {
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py(px(10. * dscale))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(title))
                        .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(desc)),
                )
                .child(ctrl)
        };
        let divider = || div().h(px(1.)).w_full().bg(theme.border);
        // Select-style control pill (static parity render).
        let select_pill = |label: String, icon: Option<&'static [u8]>| {
            let mut b = h_flex()
                .h(px(28.))
                .px(px(10.))
                .gap_1p5()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border);
            if let Some(ic) = icon {
                b = b.child(svg_icon(ic).size(px(14.)).text_color(theme.muted_foreground));
            }
            b.child(div().text_size(px(13.)).whitespace_nowrap().child(label))
                .child(svg_icon(icons::CHEVRON_DOWN).size(px(12.)).text_color(theme.muted_foreground))
                .into_any_element()
        };
        let outline_btn = |label: &'static str| {
            div()
                .flex_shrink_0()
                .h(px(28.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .child(div().text_size(px(13.)).whitespace_nowrap().child(label))
                .into_any_element()
        };
        let section_label = |t: &'static str| {
            div()
                .px_2()
                .py_1()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(t)
        };

        // ---- shared controls for ported settings sections ----
        let accent_c = rgb(0xc74614);
        let toggle = move |on: bool| {
            let mut t = div()
                .w(px(28.))
                .h(px(16.))
                .rounded_full()
                .flex()
                .items_center();
            if on {
                t = t.bg(accent_c).justify_end();
            } else {
                t = t.bg(rgb(0xd9d9d9)).justify_start();
            }
            t.child(
                div()
                    .w(px(12.))
                    .h(px(12.))
                    .rounded_full()
                    .bg(gpui::white())
                    .mx(px(2.)),
            )
            .into_any_element()
        };
        // Upstream segmented = ghost buttons gap-1; active gets a gray "secondary" fill.
        let seg = |opts: &[&'static str], active: usize| {
            let mut s = h_flex().items_center().gap(px(4.));
            for (i, o) in opts.iter().enumerate() {
                let mut c = div()
                    .h(px(32.))
                    .px(px(11.))
                    .flex()
                    .items_center()
                    .rounded(px(8.));
                if i == active {
                    c = c.bg(theme.secondary);
                }
                let tc = if i == active { theme.foreground } else { theme.muted_foreground };
                s = s.child(
                    c.child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(tc).child(*o)),
                );
            }
            s.into_any_element()
        };
        // Color swatch pill: bordered pill with color box + hex label.
        // Color-filled pill with the hex label inside, like upstream's theme swatch.
        let swatch_pill = |hex_label: &'static str, color: u32| {
            let r = ((color >> 16) & 0xff) as f32;
            let g = ((color >> 8) & 0xff) as f32;
            let b = (color & 0xff) as f32;
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let ink = if lum > 140. { rgba(0x000000aa) } else { rgba(0xffffffcc) };
            div()
                .w(px(160.))
                .h(px(26.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(rgb(color))
                .child(div().text_size(px(13.)).text_color(ink).child(hex_label))
                .into_any_element()
        };
        // Thin track slider + value label.
        let slider = |frac: f32, val: &'static str| {
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .relative()
                        .w(px(110.))
                        .h(px(14.))
                        .child(
                            div()
                                .absolute()
                                .top(px(6.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .rounded_full()
                                .bg(theme.foreground),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(1.))
                                .left(px(110f32 * frac - 6.))
                                .w(px(12.))
                                .h(px(12.))
                                .rounded_full()
                                .bg(gpui::white())
                                .border_1()
                                .border_color(theme.foreground),
                        ),
                )
                .child(div().text_xs().text_color(theme.muted_foreground).child(val))
                .into_any_element()
        };
        // Bordered number field + "px" suffix.
        let num_field = |v: &'static str| {
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .h(px(28.))
                        .w(px(44.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(div().text_size(px(13.)).child(v)),
                )
                .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("px"))
                .into_any_element()
        };
        // Miniature app-window mockup for the Appearance theme picker, matching
        // upstream ThemeModePicker (MOCKUP_COLORS + MOCKUP_LAYOUT) pixel for pixel.
        // Thumb area is 190x133 (aspect 10/7 inside border-2 + p-3px of a 200px card).
        struct MockColors { backdrop: u32, panel: u32, hbar: u32, hbar_soft: u32, card: u32, rowbar: u32, hairline: u32 }
        const MOCK_LIGHT: MockColors = MockColors { backdrop: 0xe9e9e9, panel: 0xf6f6f6, hbar: 0xcfcfcf, hbar_soft: 0xe0e0e0, card: 0xffffff, rowbar: 0xe3e3e3, hairline: 0xefefef };
        const MOCK_DARK: MockColors = MockColors { backdrop: 0x5f5f5f, panel: 0x2c2c2c, hbar: 0xa6a6a6, hbar_soft: 0x7d7d7d, card: 0x3a3a3a, rowbar: 0x707070, hairline: 0x4d4d4d };
        // Upstream MOCKUP_LAYOUT resolves percentages against the containing
        // block's width; at a 190x133 thumb the panel is 160 wide and the card
        // 132, giving the absolute values used below.
        let mock_surface = |c: &MockColors| {
            div()
                .size_full()
                .bg(rgb(c.backdrop))
                .child(
                    div().absolute().left(px(15.)).right(px(15.)).top(px(22.)).bottom_0()
                        .bg(rgb(c.panel)).rounded_t(px(8.))
                        .child(div().absolute().left(px(50.)).top(px(14.)).w(px(61.)).h(px(4.)).rounded_full().bg(rgb(c.hbar)))
                        .child(div().absolute().left(px(36.)).top(px(22.)).w(px(88.)).h(px(3.)).rounded_full().bg(rgb(c.hbar_soft)))
                        .child(div().absolute().left(px(14.)).right(px(14.)).top(px(37.)).bottom_0()
                            .bg(rgb(c.card)).rounded_t(px(6.))
                            .child(rows_card(c))),
                )
        };
        fn rows_card(c: &MockColors) -> gpui::Div {
            let mut card = div().size_full();
            for i in 0..3 {
                let y = 9. + (i as f32) * 21.5;
                card = card
                    .child(div().absolute().left(px(13.)).top(px(y)).w(px(47.)).h(px(4.)).rounded_full().bg(rgb(c.rowbar)))
                    .child(div().absolute().left(px(13.)).right(px(13.)).top(px(y + 12.3)).h(px(1.)).bg(rgb(c.hairline)));
            }
            card
        }
        let theme_fg = theme.foreground;
        let theme_card = move |kind: u8, label: &'static str, selected: bool| {
            let thumb: gpui::AnyElement = match kind {
                0 => div()
                    .size_full()
                    .child(mock_surface(&MOCK_LIGHT))
                    .child(
                        div().absolute().right_0().top_0().bottom_0().w(px(95.)).overflow_hidden()
                            .child(div().absolute().right_0().top_0().bottom_0().w(px(190.))
                                .child(mock_surface(&MOCK_DARK))),
                    )
                    .into_any_element(),
                1 => div().size_full().child(mock_surface(&MOCK_LIGHT)).into_any_element(),
                _ => div().size_full().child(mock_surface(&MOCK_DARK)).into_any_element(),
            };
            let frame = div()
                .w_full()
                .p(px(3.))
                .rounded(px(14.))
                .border_2()
                .border_color(if selected { theme_fg } else { gpui::transparent_black() })
                .child(
                    div().w(px(190.)).h(px(133.)).rounded(px(8.)).overflow_hidden().relative()
                        .child(thumb),
                );
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(6.))
                .items_center()
                .child(frame)
                .child(div().text_size(px(13.)).when(selected, |d| d.font_weight(FontWeight::MEDIUM)).child(label))
                .into_any_element()
        };

        let settings_h = window.viewport_size().height - px(47.);
        // Upstream: connectors+usage -> max-w-4xl (896), profile -> max-w-3xl
        // (768), everything else -> max-w-2xl (672), mx-auto centered.
        let settings_w = px(match self.settings_section {
            "connectors" | "usage" => 896.,
            "profile" => 768.,
            _ => 672.,
        });
        let mut settings_pane = v_flex()
            .w(settings_w)
            .h(settings_h)
            .id("settings-scroll")
            .track_scroll(&self.settings_scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.settings_scroll)
            .px_6()
            .pt(px(27.))
            .pb_8()
            .gap_5();
        {
            let mut head = h_flex().w_full().items_start().justify_between().gap_4();
            if self.settings_section == "profile" {
                head = h_flex().items_start().w_full().justify_end().gap_4();
            } else {
            head = head.child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(section_title.to_string()),
                            )
                            .when(self.settings_section == "computer", |row| {
                                row.child(
                                    div()
                                        .px_1p5()
                                        .rounded(px(5.))
                                        .border_1()
                                        .border_color(theme.border)
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child("Beta"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .line_height(px(22.))
                            .child(match &*self.settings_section {
                                "general" => "Defaults for new chats, power, time, and getting started.".to_string(),
                                "appearance" => "Customize the theme, typography, density, and time format.".to_string(),
                                "notifications" => "Choose how Synara tells you when work finishes or needs attention.".to_string(),
                                "layout" => "Choose what the sidebar, the chat top bar, and the Environment panel show.".to_string(),
                                "chats" => "Control live responses, follow-ups, review defaults, and safety confirmations.".to_string(),
                                "shortcuts" => "Change, add, or remove the shortcut for every Synara command.".to_string(),
                                "profile" => "Your local activity, streaks, and a shareable stats card.".to_string(),
                                "accounts" => "Sign in to each agent once per account. Every account gets its own tab in the model picker, its own usage limits, and its own sign-in folder.".to_string(),
                                "providers" => "Choose visible coding agents and manage their installed CLI tools.".to_string(),
                                "models" => "Choose the source control writing style, Git writing model, and custom model slugs.".to_string(),
                                "skills" => "Review reusable workflows discovered across all configured providers.".to_string(),
                                "usage" => "See remaining quota and credits for every signed-in provider.".to_string(),
                                "connectors" => "Sign in to Supabase, Linear, Notion, and other services your agents can use.".to_string(),
                                "integrations" => "Give Codex, Claude, and other local agents scoped access to Synara tasks.".to_string(),
                                "devices" => "Control this computer from your other devices, or control other computers from here.".to_string(),
                                "computer" => "Let agents see and control this computer's desktop, and check backend status.".to_string(),
                                "appsnap" => "Capture another app's frontmost window directly into a task.".to_string(),
                                "worktrees" => "Review and clean up isolated workspaces created by Synara.".to_string(),
                                "archive" => "Find and restore threads you previously archived.".to_string(),
                                "privacy" => "What Synara shares, and what it can read on this computer.".to_string(),
                                "advanced" => "Recovery tools and this browser's session.".to_string(),
                                "about" => "Version, release channel, and what changed.".to_string(),
                                _ => format!("{section_title} settings are not yet ported to the native shell."),
                            }),
                    ),
            );
            }
            if self.settings_section == "profile" {
                head = head.child(h_flex().gap_2().child(outline_btn("Share")).child(outline_btn("Edit")));
            } else if self.settings_section == "shortcuts" {
                head = head.child(
                    h_flex()
                        .id("sc-reset-all")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.apply_kb_edits(vec![json!({"type": "reset"})], cx);
                        }))
                        .child(outline_btn("\u{21ba} Reset all to defaults")),
                );
            } else {
                head = head.child(outline_btn("\u{21ba} Restore defaults"));
            }
            settings_pane = settings_pane.child(head);

            if self.settings_section == "general" {
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("New chats"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Agent and account",
                                "New chats start on this account until you pick another in the model picker. After that they reuse your most recent choice.",
                                select_pill("Codex".to_string(), Some(icons::CODEX)),
                            ))
                            .child(divider())
                            .child(srow(
                                "New threads",
                                "Pick the default workspace mode for newly created draft threads.",
                                select_pill("Local".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Power"))
                            .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Keep computer awake"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child("Uses macOS caffeinate. On: Keep this computer awake at all times. Agent: Keep this computer awake while an agent is working. Off: Let the system sleep normally."))
                                            .child(div().pt_1().text_xs().text_color(theme.muted_foreground).child(self.keep_awake_status())),
                                    )
                                    .child(Self::seg_pick(cx, "seg-keepawake", &["On", "Agent", "Off"], self.keep_awake_mode, &theme, Self::set_keep_awake)),
                            )
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Time"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Time format",
                                "System default follows your browser or OS clock preference.",
                                select_pill("System default".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Getting started"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Welcome tour",
                                "Replay the first-run setup: agents, account, iPhone pairing, and first project.",
                                outline_btn("Open welcome tour"),
                            ))
                            .child(divider())
                            .child(srow(
                                "App tour",
                                "See where Activity, Tasks, Inbox, and Code review live.",
                                div()
                                    .id("btn-replay-tour")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.clear_views();
                                        this.tour_step = Some(0);
                                        cx.notify();
                                    }))
                                    .child(outline_btn("Replay tour"))
                                    .into_any_element(),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    );
            }

            if self.settings_section == "notifications" {
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_1p5()
                        .child(section_label("Activity alerts"))
                        .child(card(
                            v_flex()
                                .child(srow(
                                    "Activity toasts",
                                    "Show an in-app toast when a chat or managed terminal agent finishes or needs input.",
                                    toggle(true),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Desktop notifications",
                                    "Show an OS notification when a chat or managed terminal agent finishes or needs input while the app is in the background. Allow browser notifications to get alerts when chats or terminal agents finish or need input in the background.",
                                    h_flex()
                                        .items_center()
                                        .gap_3()
                                        .child(div().text_size(px(13.)).child("Test"))
                                        .child(toggle(true))
                                        .into_any_element(),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Wait for subagents",
                                    "Alert once the agent and all of its background subagents have finished. Turn this off to be alerted each time the agent or one of its subagents stops.",
                                    toggle(true),
                                ))
                                .into_any_element(),
                        )),
                );
            }

            if self.settings_section == "layout" {
                let mtog = |label: &'static str, on: bool| {
                    h_flex()
                        .w(px(170.))
                        .items_center()
                        .justify_between()
                        .child(div().text_size(px(13.)).child(label))
                        .child(toggle(on))
                        .into_any_element()
                };
                let env_group = |label: &'static str,
                                 left: Vec<gpui::AnyElement>,
                                 right: Vec<gpui::AnyElement>| {
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                        .child(
                            h_flex()
                                .gap_8()
                                .child(v_flex().gap_2().children(left))
                                .child(v_flex().gap_2().children(right)),
                        )
                        .into_any_element()
                };
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar organization"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Project order",
                                        "Controls how projects are arranged in the main sidebar.",
                                        select_pill("Manual order".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Thread order",
                                        "Controls how threads are arranged inside each project in the main sidebar.",
                                        select_pill("Recently active".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Working and unread on top",
                                        "With Recently active order, move working and unread threads above the rest. The list holds still while your pointer is over it.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar sections"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Chats",
                                        "Show the standalone Chats list in the sidebar footer (chats not tied to a project).",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hubs",
                                        "Show the Hubs tab in the sidebar switcher.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Automation runs",
                                        "Show the thread each standalone automation run creates. Runs stay listed on the automation's page either way; threads owned by dedicated or heartbeat automations always stay visible.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Child threads",
                                        "Show threads an agent created from another thread nested under that thread.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Environment panel"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Open by default",
                                        "Open the chat Environment panel automatically on normal threads. When off, the panel stays closed until you open it. Your last open/close also updates this preference.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Code and status",
                                            vec![
                                                mtog("Subagents", true),
                                                mtog("Repository", true),
                                                mtog("Editor", true),
                                            ],
                                            vec![
                                                mtog("Usage", true),
                                                mtog("Pull request", true),
                                                mtog("Actions", true),
                                            ],
                                        )),
                                    )
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Context and notes",
                                            vec![
                                                mtog("Recap", true),
                                                mtog("Project instructions", false),
                                            ],
                                            vec![
                                                mtog("Pinned messages", true),
                                                mtog("Notepad", false),
                                            ],
                                        )),
                                    )
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Chat top bar"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Thread tabs",
                                        "Show open threads as tabs in the chat header. Turn off to show only the current thread's title.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hand off",
                                        "Show the Hand off button in the chat top bar. Handing off stays available from a thread's sidebar menu and the composer's model picker.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Project actions",
                                        "Show the selected project's actions in the chat top bar. When off, actions still run from their keybindings and the Environment panel's Actions section.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Opens on its own"))
                );
            }


            if self.settings_section == "shortcuts" {
                let shortcut_rows = SHORTCUT_ROWS;
                let search_field = h_flex()
                    .w_full()
                    .h(px(36.))
                    .items_center()
                    .px_3()
                    .gap_2()
                    .rounded(px(10.))
                    .bg(gpui::rgba(0x0000000a))
                    .child(svg_icon(icons::SEARCH).size(px(16.)).text_color(theme.muted_foreground))
                    .child(div().flex_1().text_size(px(13.)).text_color(theme.muted_foreground).child("Search shortcuts"))
                    .child(div().text_xs().text_color(theme.muted_foreground).child("\u{2318}K"))
                    .into_any_element();
                let mut list = v_flex();
                let mut first = true;
                for (row_idx, (title, desc, kbd, commands)) in shortcut_rows.iter().enumerate() {
                    if !first {
                        list = list.child(divider());
                    }
                    first = false;
                    let title = *title;
                    let desc = *desc;
                    let numbered = commands.contains(".jump.");
                    // Group the resolved rules of this row's commands into
                    // bindings (shared chord for normal rows, shared modifier
                    // mask for numbered families).
                    let mut bindings: Vec<(String, Vec<Value>)> = Vec::new();
                    if !self.kb_rules.is_empty() {
                        for r in &self.kb_rules {
                            let rc = r.get("command").and_then(Value::as_str).unwrap_or("");
                            if !commands.split(' ').any(|c| c == rc) {
                                continue;
                            }
                            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
                            let gk = if numbered {
                                sc_mod_mask(&sc)
                            } else {
                                sc_enc_shortcut(&sc)
                            };
                            if let Some(b) = bindings.iter_mut().find(|(g, _)| *g == gk) {
                                b.1.push(r.clone());
                            } else {
                                bindings.push((gk, vec![r.clone()]));
                            }
                        }
                    }
                    let mut right = h_flex().items_center().gap_2();
                    if self.kb_rules.is_empty() {
                        if !kbd.is_empty() {
                            right = right.child(sc_chip(*kbd, &theme));
                        }
                    } else if bindings.is_empty() {
                        right = right.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Unassigned"),
                        );
                    }
                    let nbind = bindings.len();
                    for (bi, (_g, rules)) in bindings.iter().enumerate() {
                        let label = if numbered {
                            let sc = rules[0].get("shortcut").cloned().unwrap_or(Value::Null);
                            format!("{}1\u{2013}9", sc_mod_label(&sc))
                        } else {
                            sc_fmt_label(
                                &rules[0].get("shortcut").cloned().unwrap_or(Value::Null),
                            )
                        };
                        right = right.child(sc_chip(label, &theme));
                        let repl = rules.clone();
                        let label2 = title.to_string();
                        let cmds2: Vec<String> =
                            commands.split(' ').map(|c| c.to_string()).collect();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-edit-{row_idx}-{bi}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_open(
                                        label2.clone(),
                                        cmds2.clone(),
                                        numbered,
                                        repl.clone(),
                                        cx,
                                    );
                                }))
                                .child(
                                    svg_icon(icons::PENCIL)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                        let rm = rules.clone();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-del-{row_idx}-{bi}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_remove_binding(&rm, cx);
                                }))
                                .child(
                                    svg_icon(icons::TRASH)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                        if bi + 1 == nbind {
                            let label3 = title.to_string();
                            let cmds3: Vec<String> =
                                commands.split(' ').map(|c| c.to_string()).collect();
                            right = right.child(
                                h_flex()
                                    .id(SharedString::from(format!("sc-add-{row_idx}")))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.sc_open(
                                            label3.clone(),
                                            cmds3.clone(),
                                            numbered,
                                            Vec::new(),
                                            cx,
                                        );
                                    }))
                                    .child(
                                        svg_icon(icons::PLUS)
                                            .size(px(14.))
                                            .text_color(theme.muted_foreground),
                                    ),
                            );
                        }
                    }
                    if self.kb_rules.is_empty() || bindings.is_empty() {
                        let label4 = title.to_string();
                        let cmds4: Vec<String> =
                            commands.split(' ').map(|c| c.to_string()).collect();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-rec-{row_idx}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_open(
                                        label4.clone(),
                                        cmds4.clone(),
                                        numbered,
                                        Vec::new(),
                                        cx,
                                    );
                                }))
                                .child(
                                    svg_icon(icons::PENCIL)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                    }
                    list = list.child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().text_size(px(13.)).line_height(px(18.)).font_weight(FontWeight::MEDIUM).child(title))
                                    .child(div().w_full().text_size(px(13.)).line_height(px(16.)).text_color(theme.muted_foreground).child(desc)),
                            )
                            .child(right),
                    );
                }
                settings_pane = settings_pane
                    .child(search_field)
                    .child(card(list.into_any_element()));
            }


            // ==================== remaining sections ====================
            let sdata = &self.settings_data;
            let cfg_providers: Vec<Value> = arr(get(sdata, "config"), "providers").to_vec();
            let usage_rows: Vec<Value> = arr(get(sdata, "usage"), "snapshots")
                .to_vec()
                .into_iter()
                .chain(arr(get(sdata, "usage"), "rows").to_vec())
                .chain(arr(get(sdata, "usage"), "providers").to_vec())
                .chain(if get(sdata, "usage").is_array() {
                    arr(get(sdata, "usage"), "").to_vec()
                } else {
                    Vec::new()
                })
                .collect();
            let dash = |v: &Value| div().text_size(px(13.)).text_color(theme.muted_foreground).child("\u{2014}").into_any_element();
            fn provider_display(id: &str) -> &str {
                match id {
                    "codex" => "Codex", "claudeAgent" | "claude" => "Claude", "cursor" => "Cursor",
                    "antigravity" => "Antigravity", "grok" => "Grok", "droid" => "Droid",
                    "devin" => "Devin", "opencode" => "OpenCode", "pi" => "Pi", "omp" => "Oh My Pi",
                    other => other,
                }
            }

            // ---------- Profile ----------
            if self.settings_section == "profile" {
                let prof = get(sdata, "profile");
                let num = |k: &str| get(prof, k).as_i64().map(|n| n.to_string());
                let threads_total = arr(get(sdata, "snapshot"), "threads").len()
                    .max(arr(get(sdata, "snapshot"), "items").len());
                let stat = |val: String, label: &'static str| {
                    v_flex()
                        .flex_1()
                        .items_center()
                        .gap(px(2.))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(val))
                        .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                        .into_any_element()
                };
                let ins = |label: &'static str, val: String| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .py(px(4.))
                        .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(label))
                        .child(div().text_size(px(13.)).child(val))
                        .into_any_element()
                };
                let heat_col = || {
                    v_flex()
                        .gap(px(3.))
                        .children((0..7).map(|_| {
                            div().w(px(9.)).h(px(9.)).rounded(px(2.)).bg(chip_bg).into_any_element()
                        }))
                        .into_any_element()
                };
                let months = ["Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov"];
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .w_full()
                            .items_center()
                            .gap(px(6.))
                            .py_4()
                            .child(
                                div()
                                    .w(px(44.))
                                    .h(px(44.))
                                    .rounded_full()
                                    .bg(rgb(0x34a853))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(gpui::white()).child("DE")),
                            )
                            .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("Devin"))
                            .child(div().text_xs().text_color(theme.muted_foreground).child("@devin \u{b7} Synara")),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .py_2()
                            .child(stat(num("lifetimeTokens").unwrap_or_else(|| "\u{2014}".into()), "Lifetime tokens"))
                            .child(stat(num("peakDay").unwrap_or_else(|| "\u{2014}".into()), "Peak day"))
                            .child(stat(num("totalPrompts").unwrap_or_else(|| "0".into()), "Total prompts"))
                            .child(stat(num("currentStreak").map(|s| format!("{s} days")).unwrap_or_else(|| "0 days".into()), "Current streak"))
                            .child(stat(num("longestStreak").map(|s| format!("{s} days")).unwrap_or_else(|| "0 days".into()), "Longest streak")),
                    )
                    .child(section_label("Activity"))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(h_flex().gap(px(3.)).children((0..44).map(|_| heat_col())))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .children(months.map(|m| {
                                        div().text_xs().text_color(theme.muted_foreground).child(m).into_any_element()
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_6()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(section_label("Activity insights"))
                                    .child(ins("Most used provider", "\u{2014}".into()))
                                    .child(ins("Most used reasoning", "\u{2014}".into()))
                                    .child(ins("Most active hour", "\u{2014}".into()))
                                    .child(ins("Most worked project", "\u{2014}".into()))
                                    .child(ins("Skills explored", "0".into()))
                                    .child(ins("Total skills used", "0".into()))
                                    .child(ins("Total threads", format!("{threads_total}"))),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(section_label("Most used plugins"))
                                    .child(div().py(px(4.)).text_size(px(13.)).text_color(theme.muted_foreground).child("No skills or agents used yet.")),
                            ),
                    )
                    .child(section_label("Model usage"))
                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Share of turns."))
                    .child(div().pt_1().text_size(px(13.)).text_color(theme.muted_foreground).child("No model activity yet."));
            }

            // ---------- Accounts ----------
            if self.settings_section == "accounts" {
                let acct_icon = |pid: &str| match pid {
                    "devin" => icons::P_DEVIN, "pi" => icons::P_PI, "codex" => icons::CODEX,
                    "claudeAgent" | "claude" => icons::P_CLAUDE, "cursor" => icons::P_CURSOR,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "opencode" => icons::P_OPENCODE,
                    "antigravity" => icons::P_ANTIGRAVITY, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let shown: Vec<(&'static str, &'static str, &'static str, &'static str, &'static str)> = if cfg_providers.is_empty() {
                    vec![
                        ("devin", "Devin", "Authenticated \u{b7} Devin API Key", "Pro", "Daily 100% left \u{b7} Weekly 93% left"),
                        ("pi", "Pi", "Available", "", ""),
                    ]
                } else {
                    cfg_providers.iter().filter_map(|p| {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() { str_at(p, "driver") } else { pid };
                        match &*pid {
                            "devin" => Some(("devin","Devin","Authenticated \u{b7} Devin API Key","Pro","Daily 100% left \u{b7} Weekly 93% left")),
                            "pi" => Some(("pi","Pi","Available","","")),
                            _ => None,
                        }
                    }).collect()
                };
                for (pid, name, status, plan, quota) in shown {
                    let _ = pid;
                    settings_pane = settings_pane
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(section_label(name))
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("\u{2b06} Import directory"))
                                        .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("+ Add account")),
                                ),
                        )
                        .child(card(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_3()
                                .px_3()
                                .py(px(10.))
                                .child(svg_icon(acct_icon(pid)).size(px(18.)).text_color(theme.foreground))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(1.))
                                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name))
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0x22a865)))
                                                .child(div().text_xs().text_color(theme.muted_foreground).child(status)),
                                        )
                                        .children(if plan.is_empty() { vec![] } else {
                                            vec![div().text_xs().text_color(theme.muted_foreground).child(plan)]
                                        })
                                        .children(if quota.is_empty() { vec![] } else {
                                            vec![div().text_xs().text_color(theme.muted_foreground).child(quota)]
                                        }),
                                )
                                .child(toggle(true))
                                .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                .into_any_element(),
                        ));
                }
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "Not installed: Codex, Claude, Cursor, Antigravity, Grok, Droid, OpenCode, Oh My Pi. Install them under Providers to add their accounts."))
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "A chat stays on the account it started with. Pick the account for a new chat in the model picker. CLI versions, updates, and binary paths live in Providers.")),
                );
            }

            // ---------- Providers ----------
            if self.settings_section == "providers" {
                let p_icon = |pid: &str| match pid {
                    "codex" => icons::CODEX, "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR, "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE, "pi" => icons::P_PI, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let mut rows: Vec<(String, String, String, bool)> = Vec::new();
                if cfg_providers.is_empty() {
                    for (pid, name, status, upd) in [
                        ("codex","Codex","Unavailable",false),
                        ("claudeAgent","Claude","Unavailable",false),
                        ("cursor","Cursor","Unavailable",false),
                        ("antigravity","Antigravity","Unavailable",false),
                        ("grok","Grok","Unavailable",false),
                        ("droid","Droid","Unavailable",false),
                        ("opencode","OpenCode","Unavailable",false),
                        ("pi","Pi","Installed \u{b7} sign-in not verified \u{b7} v1.4.0 -> v1.6.0",true),
                        ("devin","Devin","Connected \u{b7} Current v3000.14.30821",false),
                        ("omp","Oh My Pi","Unavailable",false),
                    ] {
                        rows.push((pid.to_string(), name.to_string(), status.to_string(), upd));
                    }
                } else {
                    for p in &cfg_providers {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() { str_at(p, "driver") } else { pid };
                        if pid.is_empty() { continue; }
                        let avail = get(p, "available").as_bool().unwrap_or(false);
                        let ver = str_at(p, "version");
                        let status = if avail {
                            if ver.is_empty() { "Installed".to_string() } else { format!("Installed \u{b7} {ver}") }
                        } else {
                            "Unavailable".to_string()
                        };
                        let upd = get(p, "updateAvailable").as_bool().unwrap_or(false);
                        rows.push((pid.to_string(), provider_display(pid).to_string(), status, upd));
                    }
                }
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Agents"))
                            .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("Refresh status")),
                    )
                    .child(card(
                        v_flex()
                            .children(rows.iter().enumerate().flat_map(|(i, (pid, name, status, upd))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    .py(px(10.))
                                    .child(div().w(px(16.)).text_size(px(13.)).text_color(theme.muted_foreground).child("\u{283f}"))
                                    .child(svg_icon(p_icon(pid)).size(px(16.)).text_color(theme.foreground))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name.clone()))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(status.clone())),
                                    )
                                    .children(if *upd {
                                        vec![div()
                                            .px_2()
                                            .py_1()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .text_xs()
                                            .child("\u{2b07} Update")
                                            .into_any_element()]
                                    } else { vec![] })
                                    .child(toggle(true))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element();
                                if i > 0 {
                                    vec![divider().into_any_element(), row]
                                } else {
                                    vec![row]
                                }
                            }))
                            .into_any_element(),
                    ))
                    .child(section_label("Performance"))
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Keep Synara responsive"))
                                            .child(toggle(true)),
                                    )
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Give agent processes and their commands a moderately lower CPU priority when the machine is busy."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Applies to newly launched agent processes. Restart existing sessions to apply consistently.")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Updates"))
                    .child(card(
                        v_flex()
                            .child(srow("Automatic CLI update checks",
                                "Check Codex, Claude, and other provider CLIs for newer versions in the background.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Provider updates"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Review installed provider tools that Synara can safely update."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "1 update available")),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Models ----------
            if self.settings_section == "models" {
                settings_pane = settings_pane
                    .child(section_label("Generation defaults"))
                    .child(card(
                        v_flex()
                            .child(srow("Source control writing style", "In each project, matches recent change descriptions and change request titles.", select_pill("Repository conventions".into(), None)))
                            .child(divider())
                            .child(srow("Add Synara attribution", "Adds a \"Created with Synara\" link to the end of pull request descriptions that Synara creates. Commits and branch names are never changed.", toggle(true)))
                            .child(divider())
                            .child(srow("Git writing model", "Used for generated commit messages, PR titles, and branch names.", select_pill("GPT-6 Luna".into(), None)))
                            .into_any_element(),
                    ))
                    .child(section_label("Custom models"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap_2()
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Saved model slugs"))
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child("Add custom model slugs for supported providers."))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .child(select_pill("Codex".into(), None))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("gpt-6.7-codex-ultra-preview")),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1p5()
                                            .rounded_full()
                                            .bg(rgb(0x26251f))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(gpui::white())
                                            .child("+ Add"),
                                    ),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Skills ----------
            if self.settings_section == "skills" {
                let skills = arr(get(sdata, "skills"), "skills").to_vec();
                let skills = if skills.is_empty() { arr(get(sdata, "skills"), "items").to_vec() } else { skills };
                settings_pane = settings_pane
                    .child(section_label("Portable skills"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Synara skills folder"))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} of {} skills enabled", skills.len(), skills.len()))),
                            )
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Skills placed here are available on every provider. When a provider already ships its own copy of a skill, that copy is used; otherwise Synara's copy is the fallback."))
                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child("~/.synara/skills"))
                            .into_any_element(),
                    ))
                    .child(section_label("Skills"));
                if skills.is_empty() {
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No skills found"))
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Add a skill folder containing a SKILL.md to the Synara skills folder above, or install skills for any supported provider."))
                            .into_any_element(),
                    ));
                }
                for s in &skills {
                    let nm = str_at(s, "name").to_string();
                    let desc = str_at(s, "description").to_string();
                    let pv = str_at(s, "provider").to_string();
                    let desc = if desc.is_empty() { provider_display(&pv).to_string() } else { desc };
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(desc))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Usage ----------
            if self.settings_section == "usage" {
                let p_icon2 = |pid: &str| match pid {
                    "codex" => icons::CODEX, "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR, "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE, "pi" => icons::P_PI, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let sidebar_rows: [(&'static str, &'static str, bool); 9] = [
                    ("codex","Codex",true), ("claudeAgent","Claude",true), ("cursor","Cursor",false),
                    ("antigravity","Antigravity",false), ("grok","Grok",false), ("droid","Droid",false),
                    ("opencode","OpenCode",false), ("pi","Pi",false), ("devin","Devin",false),
                ];
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(6.))
                                    .bg(chip_bg)
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Overview"),
                            )
                            .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("Analytics")),
                    )
                    .child(section_label("Sidebar \u{b7} up to 2 accounts"))
                    .child(card(
                        v_flex()
                            .children(sidebar_rows.iter().enumerate().flat_map(|(i, (pid, name, on))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    .py(px(9.))
                                    .child(svg_icon(p_icon2(pid)).size(px(16.)).text_color(theme.foreground))
                                    .child(div().flex_1().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name.to_string()))
                                    .child(toggle(*on))
                                    .into_any_element();
                                if i > 0 { vec![divider().into_any_element(), row] } else { vec![row] }
                            }))
                            .into_any_element(),
                    ))
                    .child(section_label("Ring"))
                    .child(card(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(div().flex_1().min_w_0().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Show both limits as two rings, or a single ring for one of them."))
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Both"))
                                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("5h"))
                                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Weekly")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Usage popovers"))
                    .child(card(
                        v_flex()
                            .child(srow("Show details by default",
                                "Open the details below the limits. When off, they stay behind the Details toggle; your last toggle also updates this preference.",
                                toggle(false)))
                            .child(divider())
                            .child(srow("Banked resets",
                                "Include Codex banked resets in the details.",
                                toggle(true)))
                            .into_any_element(),
                    ));
            }

            // ---------- Connectors ----------
            if self.settings_section == "connectors" {
                let search = h_flex()
                    .w_full()
                    .h(px(36.))
                    .pl(px(14.))
                    .pr_4()
                    .gap(px(8.))
                    .rounded_full()
                    .border_1()
                    .border_color(theme.border)
                    .bg(soft_bg)
                    .items_center()
                    .child(svg_icon(icons::SEARCH).size(px(14.)).text_color(theme.muted_foreground))
                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Search connectors"))
                    .into_any_element();
                let dark_btn = |label: &'static str| {
                    h_flex()
                        .h(px(36.))
                        .px_4()
                        .gap_1p5()
                        .items_center()
                        .rounded_full()
                        .bg(rgb(0x26251f))
                        .child(svg_icon(icons::PLUS).size(px(16.)).text_color(gpui::white()))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(gpui::white()).child(label))
                        .into_any_element()
                };
                let catalog: [(&'static str, &'static str); 3] = [
                    ("popular", "Popular"),
                    ("developer", "Developer tools"),
                    ("productivity", "Productivity"),
                ];
                let items: [(&'static str, &'static str, &'static str, &'static str); 18] = [
                    ("popular","Supabase","Query Postgres, manage projects, and deploy Edge Functions.","supabase"),
                    ("popular","Vercel","Manage projects and deployments, and read deployment logs.","vercel"),
                    ("popular","Slack","Search, read, and post in your workspace. Needs your own Slack app.","slack"),
                    ("popular","Linear","Find, create, and update issues, projects, and comments.","linear"),
                    ("popular","Notion","Search, read, and edit pages and databases.","notion"),
                    ("popular","Stripe","Work with customers, payments, and Stripe docs.","stripe"),
                    ("developer","WorkOS","Manage users, organizations, SSO, and AuthKit in your dashboard.","workos"),
                    ("developer","Convex","Start Convex apps and get setup and scaling guidance.","convex"),
                    ("developer","Cloudflare","Build with Workers, KV, R2, and D1 on your account.","cloudflare"),
                    ("developer","Sentry","Investigate errors, issues, and releases.","sentry"),
                    ("developer","Neon","Manage serverless Postgres projects and branches.","neon"),
                    ("developer","PostHog","Query product analytics, feature flags, and errors.","posthog"),
                    ("developer","Netlify","Create, deploy, and manage Netlify sites.","netlify"),
                    ("developer","Railway","Deploy and manage Railway projects and services.","railway"),
                    ("developer","Prisma","Manage Prisma Postgres databases and backups.","prisma"),
                    ("developer","Hugging Face","Search models, datasets, Spaces, and papers.","huggingface"),
                    ("productivity","Atlassian","Work with Jira issues and Confluence pages.","atlassian"),
                    ("productivity","Todoist","Find, add, and complete tasks and projects.","todoist"),
                ];
                settings_pane = settings_pane
                    .child(search)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(dark_btn("Add custom"))
                            .child(
                                h_flex()
                                    .h(px(36.))
                                    .px_4()
                                    .items_center()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Add local server")),
                            ),
                    )
                    ;
                let mut catalog_grid = v_flex().w_full().gap(px(2.));
                let mut first_cat = true;
                for (cat, cat_name) in &catalog {
                    let label = section_label(cat_name);
                    catalog_grid = catalog_grid.child(if first_cat {
                        first_cat = false;
                        label
                    } else {
                        label.mt(px(16.))
                    });
                    let cat_items: Vec<_> = items.iter().filter(|(c, ..)| c == cat).collect();
                    for chunk in cat_items.chunks(2) {
                        let mut row = h_flex().w_full().gap(px(24.));
                        for (_, name, desc, icon) in chunk {
                            row = row.child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_3()
                                    .py(px(10.))
                                    .child(
                                        div()
                                            .w(px(44.))
                                            .h(px(44.))
                                            .flex_shrink_0()
                                            .rounded(px(14.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(gpui::white())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                            gpui::img(
                                                std::path::PathBuf::from(
                                                    format!("{}/assets/icons/brands/{}.svg", env!("CARGO_MANIFEST_DIR"), *icon),
                                                ),
                                            )
                                            .size(px(20.)),
                                        ),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(*name))
                                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(*desc)),
                                    )
                                    .child(div().px_2().text_size(px(13.)).text_color(theme.muted_foreground).child("+")),
                            );
                        }
                        if chunk.len() == 1 {
                            row = row.child(div().flex_1());
                        }
                        catalog_grid = catalog_grid.child(row);
                    }
                }
                settings_pane = settings_pane.child(catalog_grid);
            }

            // ---------- External agents (integrations) ----------
            if self.settings_section == "integrations" {
                let dark_btn = |label: &'static str| {
                    div()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(rgb(0x26251f))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .child(label)
                        .into_any_element()
                };
                let exts = arr(get(sdata, "external"), "integrations").to_vec();
                let exts = if exts.is_empty() { arr(get(sdata, "external"), "items").to_vec() } else { exts };
                settings_pane = settings_pane
                    .child(section_label("Connect a coding agent"))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Name"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "How this connection appears in Synara. Works with Codex, Claude, and any other MCP-capable agent.")),
                                    )
                                    .child(
                                        div()
                                            .w(px(160.))
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().child("Coding agent")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Access all of Synara",
                                "The agent can discover and work in every project, including ones you add later. Turn off to pick specific projects.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Advanced permissions"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Optional access for existing tasks, shared checkouts, or execution without approvals. The safe defaults are recommended.")),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Review"))
                                            .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground)),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Create connection"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "The connection lasts 30 days and can be revoked at any time. The next screen gives you one prompt to paste into your agent.")),
                                    )
                                    .child(dark_btn("Create connection"))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Connected agents"));
                if exts.is_empty() {
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No connected agents"))
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Connect Codex, Claude, or another local MCP agent to create and follow Synara tasks."))
                            .into_any_element(),
                    ));
                }
                for e in &exts {
                    let nm = str_at(e, "name").to_string();
                    let st = str_at(e, "status").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(st))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Devices (connections) ----------
            if self.settings_section == "devices" {
                settings_pane = settings_pane
                    .child(section_label("Connections"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(1.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Unavailable in this build"))
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Remote connections require the Node 24 server runtime."))
                            .into_any_element(),
                    ));
            }

            // ---------- Computer use ----------
            if self.settings_section == "computer" {
                let comp = get(sdata, "computer");
                let avail = get(comp, "available").as_bool().unwrap_or(false);
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("Computer control"))
                            .child(toggle(false)),
                    )
                    .child(div().w_full().px_2().text_size(px(13.)).text_color(theme.muted_foreground).child(
                        "Enable Computer by default in any chat. Leave this off and use /computer-use for one request without adding Computer tools to ordinary turns."))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0xd43d3d)))
                                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(if avail { "Computer control is available" } else { "Computer control is unavailable" })),
                                            )
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Computer requires a connected Synara desktop host, which owns native access on that computer.")),
                                    )
                                    .child(outline_btn("Set up"))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Cursor colors"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "The agent pointer is stock monochrome by default \u{2014} like a normal pointer. Custom colors apply to new computer sessions.")),
                                    )
                                    .child(seg(&["Stock", "Custom"], 0))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Preview"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Show the live preview the first time an agent acts on the desktop in a chat. Compact keeps it small and glanceable; Large gives it the full wide card.")),
                                    )
                                    .child(toggle(true))
                                    .child(seg(&["Compact", "Large"], 0))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Getting started"))
                            .child(div().px_2().py_1().text_size(px(13.)).text_color(theme.muted_foreground).child("\u{2304} Hide guide")),
                    )
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Ask for a task"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Type /computer-use followed by your task, for example: \u{201c}/computer-use open Calculator and calculate 123 \u{d7} 45.\u{201d} This enables Computer for that request only. The default setting below can enable it on every turn.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Approve the task"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "If asked, approve Computer for the task. Use the permission guide when desktop access is missing. Synara may still ask before consequential actions.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Follow and stop"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Watch the preview while the agent works. Use Stop in the chat to interrupt the task. Closing the preview only hides it.")),
                            )
                            .into_any_element(),
                    ))
                    .child(div().pt_1().child(outline_btn("Got it")))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Recent Computer actions"))
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("\u{203a} Show history")),
                    );
            }

            // ---------- AppSnap ----------
            if self.settings_section == "appsnap" {
                settings_pane = settings_pane
                    .child(card(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(svg_icon(icons::S_SCREEN_CAPTURE).size(px(18.)).text_color(theme.muted_foreground))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Take an AppSnap to show your agent another app's window"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Press your two-key shortcut while any app is frontmost. Synara captures that window as an image, brings itself forward, and attaches the snap to a task composer \u{2014} the capture stays on this device until you send the message."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "AppSnap requires the Synara desktop app on macOS.")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Capture"))
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .w_full()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .min_w_0()
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Enable AppSnap"))
                                            .child(toggle(false)),
                                    )
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Run the capture listener in the background while Synara is open."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Available in the Synara desktop app")),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Shortcut"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Choose exactly two keys: one modifier and one other key. Synara checks its own bindings and asks macOS whether another app already owns the shortcut before saving it.")),
                                    )
                                    .child(
                                        v_flex()
                                            .items_end()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).child("\u{2325} left + \u{2325} right"))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("Current shortcut")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Destination",
                                "Snaps join the task you interacted with in the last minute, and consecutive snaps stay together. Otherwise Synara opens a fresh task with the capture attached.",
                                select_pill("Automatic".into(), None)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Capture sound"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Play a short shutter cue when a window is captured.")),
                                    )
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Preview"))
                                    .child(toggle(true))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Worktrees ----------
            if self.settings_section == "worktrees" {
                let wts = arr(get(sdata, "worktrees"), "worktrees").to_vec();
                let wts = if wts.is_empty() { arr(get(sdata, "worktrees"), "items").to_vec() } else { wts };
                settings_pane = settings_pane
                    .child(section_label("Defaults"))
                    .child(card(
                        srow("Delete worktree on archive",
                            "After Archive's Undo period, remove a clean worktree only if the task has stopped and no other task uses it. Its branch remains available for recovery.",
                            toggle(false))
                        .into_any_element(),
                    ));
                if wts.is_empty() {
                    settings_pane = settings_pane.child(
                        div()
                            .w_full()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(theme.border)
                            .py(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("No app-managed worktrees found yet.")),
                    );
                }
                for w in &wts {
                    let nm = str_at(w, "name").to_string();
                    let path = str_at(w, "path").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(path))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Archive ----------
            if self.settings_section == "archive" {
                settings_pane = settings_pane.child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .py(px(40.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(svg_icon(icons::TRASH).size(px(20.)).text_color(theme.muted_foreground))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No archived threads"))
                        .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                            "Archived threads will appear here and can be restored to the sidebar.")),
                );
            }

            // ---------- Privacy ----------
            if self.settings_section == "privacy" {
                // Upstream renders nothing extra in this build (Safari onboarding and
                // product-analytics rows are desktop/remote-gated) — title + subtitle only.
            }

            // ---------- Advanced ----------
            if self.settings_section == "advanced" {
                settings_pane = settings_pane
                    .child(section_label("Developer tools"))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Recovery tools"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Rebuild local project indexes without clearing existing chats when the local state gets out of sync."))
                                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                                "Visible because projects exist but no chat history is currently available.")),
                                    )
                                    .child(outline_btn("Repair state"))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(8.))
                                    .child(div().text_size(px(13.)).child("What this does"))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- About ----------
            if self.settings_section == "about" {
                settings_pane = settings_pane
                    .child(section_label("About"))
                    .child(card(
                        v_flex()
                            .child(srow("Version", "Current application version.",
                                div().text_size(px(13.)).text_color(theme.muted_foreground).child("1.6.4").into_any_element()))
                            .child(divider())
                            .child(srow("Release history",
                                "A running log of every update, newest first. Same notes the post-update dialog shows, kept here so you can revisit them any time.",
                                outline_btn("View release history")))
                            .into_any_element(),
                    ));
            }

            if self.settings_section == "appearance" {
                // Plain row without a description line.
                let prow = |label: &'static str, ctrl: gpui::AnyElement| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py(px(10. * dscale))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(label))
                        .child(ctrl)
                        .into_any_element()
                };
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_1p5()
                        .child(section_label("Theme"))
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    div().id("theme-card-system").cursor_pointer()
                                        .child(theme_card(0, "System", self.appearance_mode == 0))
                                        .on_click(cx.listener(|this, _ev, w, cx| this.set_appearance_mode(0, w, cx))),
                                )
                                .child(
                                    div().id("theme-card-light").cursor_pointer()
                                        .child(theme_card(1, "Light", self.appearance_mode == 1))
                                        .on_click(cx.listener(|this, _ev, w, cx| this.set_appearance_mode(1, w, cx))),
                                )
                                .child(
                                    div().id("theme-card-dark").cursor_pointer()
                                        .child(theme_card(2, "Dark", self.appearance_mode == 2))
                                        .on_click(cx.listener(|this, _ev, w, cx| this.set_appearance_mode(2, w, cx))),
                                ),
                        ),
                );
                for dark in [false, true] {
                    let (name, desc, acc, bgc, fgc, al, bl, fl) = if dark {
                        ("Dark theme", "Used when your system switches to dark.", 0xf2612d, 0x111111, 0xfcfcfc, "#F2612D", "#111111", "#FCFCFC")
                    } else {
                        ("Light theme", "System is currently using this light slot.", 0xc74614, 0xffffff, 0x0d0d0d, "#C74614", "#FFFFFF", "#0D0D0D")
                    };
                    let aa_chip = if dark {
                        div().rounded(px(4.)).bg(rgb(0x111111)).px(px(3.))
                            .child(div().text_xs().text_color(gpui::white()).child("Aa"))
                            .into_any_element()
                    } else {
                        div().text_size(px(13.)).text_color(accent_c).child("Aa").into_any_element()
                    };
                    let mut sel = h_flex()
                        .w_full()
                        .mt_1()
                        .mb_2()
                        .h(px(36.))
                        .items_center()
                        .justify_between()
                        .px_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border);
                    if dark {
                        sel = sel.bg(rgb(0x111111)).child(div().text_size(px(13.)).text_color(gpui::white()).child("Synara"));
                    } else {
                        sel = sel.child(div().text_size(px(13.)).child("Synara"));
                    }
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pt_3()
                                    .pb_1()
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Import"))
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Copy"))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(aa_chip)
                                                    .child(div().text_size(px(13.)).child("Synara")),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pb_2()
                                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(desc))
                                    .when(dark, |d| {
                                        d.child(
                                            h_flex()
                                                .id("use-dark-theme")
                                                .items_center()
                                                .gap_2()
                                                .cursor_pointer()
                                                .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Use dark theme"))
                                                .child(toggle(self.appearance_mode == 2))
                                                .on_click(cx.listener(|this, _ev, w, cx| {
                                                    let next = if this.appearance_mode == 2 { 0 } else { 2 };
                                                    this.set_appearance_mode(next, w, cx);
                                                })),
                                        )
                                    }),
                            )
                            .child(
                                div().w_full().px_3().child(
                                    sel.child(div().text_size(px(13.)).text_color(accent_c).child("Accent preview")),
                                ),
                            )
                            .child(divider())
                            .child(prow("Accent", swatch_pill(al, acc)))
                            .child(divider())
                            .child(prow("Background", swatch_pill(bl, bgc)))
                            .child(divider())
                            .child(prow("Foreground", swatch_pill(fl, fgc)))
                            .child(divider())
                            .child(prow(
                                "UI font",
                                v_flex()
                                    .items_end()
                                    .gap_1()
                                    .child(select_pill("System default".to_string(), None))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Use system UI font is on; theme fonts are not applied."))
                                    .into_any_element(),
                            ))
                            .child(divider())
                            .child(prow("Code font", select_pill("\"JetBrains Mono\"".to_string(), None)))
                            .child(divider())
                            .child(prow("Window", seg(&["Solid", "Translucent"], 1)))
                            .child(divider())
                            .child(prow("Sidebar only", toggle(false)))
                            .child(divider())
                            .child(prow("Opacity", slider(0.98, "98%")))
                            .child(divider())
                            .child(prow(
                                "Blur",
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Auto"))
                                    .child(slider(0.64, "64"))
                                    .into_any_element(),
                            ))
                            .child(divider())
                            .child(prow("Contrast", slider(0.0, "0")))
                            .into_any_element(),
                    ));
                }
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Effects"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "ASCII background",
                                        "Show the animated ASCII field on empty chats. Reduced motion keeps the field still.",
                                        div().id("ascii-bg-toggle").cursor_pointer()
                                            .child(toggle(self.show_ascii_bg))
                                            .on_click(cx.listener(|this, _ev, _w, cx| this.toggle_ascii_bg(cx)))
                                            .into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Typography and spacing"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Use system UI font",
                                        "Ignore the theme's custom UI font and render the interface with the native system font (SF Pro on macOS).",
                                        toggle(true),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "UI density",
                                        "Control spacing in the sidebar, composer, chat gutters, and settings rows without changing font size.",
                                        Self::seg_pick(cx, "seg-density", &["Compact", "Comfortable", "Spacious"], self.ui_density as usize, &theme, Self::set_ui_density),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Chat width",
                                        "Control how wide the chat column grows. Wide and Full give tables and wide content more room.",
                                        Self::seg_pick(cx, "seg-chatw", &["Standard", "Wide", "Full"], self.chat_width_mode as usize, &theme, Self::set_chat_width),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Base font size",
                                        "Adjust the app text base in pixels. Chat and UI typography scale proportionally from this value.",
                                        num_field("13"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font size",
                                        "Adjust terminal text independently from the app and chat font size.",
                                        num_field("12"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font",
                                        "Type any monospace font installed on this device (e.g. Fira Code). Leave empty for the default. Fonts that aren't installed fall back to the system monospace.",
                                        select_pill("Default (JetBrains Mono)".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Font smoothing",
                                        "Use macOS-style antialiasing for lighter, crisper text rendering.",
                                        toggle(true),
                                    ))
                                    .into_any_element(),
                            )),
                    );
            }
        }
        // Center the content column at upstream max-w-2xl (672px); upstream's
        // scrollbar gutter shifts the center ~6px left, so reserve it.
        let settings_pane = h_flex()
            .flex_1()
            .w_full()
            .justify_center()
            .pr(px(13.))
            .child(settings_pane);
        if std::env::var("SYNARA_SCROLL_BOTTOM").is_ok() {
            self.settings_scroll.scroll_to_bottom();
        }

        // ---- settings sidebar nav (replaces project sidebar) ----
        let mut settings_nav_el = v_flex().flex_1().h_full().px(px(6.)).pt(px(12.)).pb(px(6.)).gap(px(2.));
        settings_nav_el = settings_nav_el.child(
            div()
                .px_2()
                .pb_2()
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Settings"),
        );
        settings_nav_el = settings_nav_el.child(
            h_flex()
                .mx_1()
                .mb_2()
                .h(px(30.))
                .pl(px(14.))
                .pr_2()
                .gap(px(8.))
                .items_center()
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .bg(soft_bg)
                .child(svg_icon(icons::SEARCH).size(px(14.)).text_color(theme.muted_foreground))
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&self.settings_search).appearance(false)),
                ),
        );
        let mut nav_list = v_flex()
            .h(settings_h - px(100.))
            .id("settings-nav-scroll")
            .track_scroll(&self.settings_nav_scroll)
            .overflow_y_scroll()
            .gap(px(2.));
        let nav_row = |id: &'static str, label: &'static str, icon: &'static [u8], badge: Option<&'static str>, cx: &mut gpui::Context<Self>| {
            let active = id == self.settings_section;
            let mut row = h_flex()
                .id(ElementId::Name(format!("setnav-{id}").into()))
                .flex_shrink_0()
                .h(px(28.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .cursor_pointer()
                .child(svg_icon(icon).size(px(16.)).text_color(if active { theme.foreground } else { theme.muted_foreground }))
                .child(div().text_size(px(13.)).whitespace_nowrap().child(label));
            if let Some(b) = badge {
                row = row.child(
                    div()
                        .ml_auto()
                        .rounded_full()
                        .px(px(6.))
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .border_1()
                        .border_color(theme.border)
                        .child(b),
                );
            }
            if active {
                row = row.bg(sel_fill);
            }
            row.on_click(cx.listener(move |this, _ev, w, cx| {
                this.settings_section = id;
                this.settings_search.update(cx, |s, cx| s.set_value("", w, cx));
                if this.settings_data.is_null() {
                    this.fetch_settings(cx);
                }
                cx.notify();
            }))
        };
        let nav_query = self.settings_search.read(cx).value().trim().to_lowercase();
        if nav_query.is_empty() {
            for (group, items) in settings_nav {
                if !group.is_empty() {
                    nav_list = nav_list.child(
                        div()
                            .flex_shrink_0()
                            .mt(px(5.))
                            .px_2()
                            .py_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(*group),
                    );
                }
                for (id, label, icon, badge) in *items {
                    nav_list = nav_list.child(nav_row(id, label, icon, *badge, cx));
                }
            }
        } else {
            let mut any = false;
            for (_group, items) in settings_nav {
                for (id, label, icon, badge) in *items {
                    if !label.to_lowercase().contains(nav_query.as_str()) {
                        continue;
                    }
                    any = true;
                    nav_list = nav_list.child(nav_row(id, label, icon, *badge, cx));
                }
            }
            if !any {
                nav_list = nav_list.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching settings"),
                );
            }
        }
        settings_nav_el = settings_nav_el.child(nav_list);

        // ==================== rail route views ====================
        // Segmented pill chip — upstream: muted rounded track, white active.
        let seg_chip = |id: &'static str, label: &'static str, active: bool| {
            let d = div()
                .id(ElementId::Name(id.into()))
                .h(px(24.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(if active {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(label),
                );
            if active { d.bg(theme.background) } else { d }
        };

        // /tasks — rail only: List|Kanban toggle top-right, centered column.
        let tasks_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .pt(px(9.))
                    .pr(px(19.))
                    .child(div().flex_1())
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(soft_bg)
                            .child(seg_chip("seg-tasks-list", "List", true).on_click(cx.listener(
                                |this, _e, _w, cx| this.open_route(1, cx),
                            )))
                            .child(seg_chip("seg-tasks-kanban", "Kanban", false).on_click(
                                cx.listener(|this, _e, _w, cx| this.open_route(2, cx)),
                            )),
                    ),
            )
            .child(
                // Upstream: a centered fixed-width column, contents
                // left-aligned; add-task row is a faint borderless row.
                v_flex()
                    .flex_1()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(560.))
                            .pt(px(36.))
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Tasks"),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("Add anything you need to do"),
                            )
                            .child(
                                h_flex()
                                    .mt(px(36.))
                                    .h(px(28.))
                                    .gap(px(10.))
                                    .items_center()
                                    .child(
                                        div()
                                            .size(px(14.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(theme.muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child("Add a task, or press Tab to hand it to an agent"),
                                    ),
                            ),
                    ),
            );

        // /kanban — header strip + one column per project of task cards
        // (threads are the upstream kanban items; "Chats" groups chat-kind).
        let live = |t: &Value| get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null();
        let mut kanban_cols = h_flex().px_4().gap_4().items_start();
        let mut kanban_count = 0usize;
        {
            let snap = self.snapshot.clone().unwrap_or(Value::Null);
            let kb_projects = arr(&snap, "projects");
            let kb_threads = arr(&snap, "threads");
            let chat_ids: Vec<String> = kb_projects
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
            let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
            for p in kb_projects {
                if str_at(p, "kind") == "chat" {
                    continue;
                }
                let pid = str_at(p, "id");
                let mut kids: Vec<Value> = kb_threads
                    .iter()
                    .filter(|t| str_at(t, "projectId") == pid && live(t))
                    .cloned()
                    .collect();
                kids.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
                if !kids.is_empty() {
                    groups.push((str_at(p, "title").to_string(), kids));
                }
            }
            let mut chats: Vec<Value> = kb_threads
                .iter()
                .filter(|t| {
                    live(t)
                        && (chat_ids.iter().any(|id| id == str_at(t, "projectId"))
                            || !kb_projects
                                .iter()
                                .any(|p| str_at(p, "id") == str_at(t, "projectId")))
                })
                .cloned()
                .collect();
            chats.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
            if !chats.is_empty() {
                groups.push(("Chats".to_string(), chats));
            }
            kanban_count = groups.iter().map(|g| g.1.len()).sum();
            for (name, kids) in groups {
                let n = kids.len();
                let mut col = v_flex().w(px(288.)).gap(px(8.));
                col = col.child(
                    h_flex()
                        .items_center()
                        .px(px(2.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .ml(px(4.))
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(format!("{n}")),
                        )
                        .child(div().flex_1())
                        .child(
                            svg_icon(icons::PLUS)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        ),
                );
                for t in &kids {
                    let title = str_at(t, "title").to_string();
                    let branch = str_at(t, "branch");
                    let mut when = rel_time(str_at(t, "updatedAt"));
                    if when.is_empty() {
                        when = rel_time(str_at(t, "createdAt"));
                    }
                    let mut meta = h_flex().gap(px(8.)).items_center().child(
                        svg_icon(icons::CODEX)
                            .size(px(12.))
                            .text_color(theme.muted_foreground),
                    );
                    if !branch.is_empty() {
                        meta = meta.child(
                            h_flex()
                                .gap(px(2.))
                                .items_center()
                                .child(
                                    svg_icon(icons::S_BRANCH_SIMPLE)
                                        .size(px(11.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .child(branch.to_string()),
                                ),
                        );
                    }
                    meta = meta.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(when),
                    );
                    meta = meta.child(
                        h_flex()
                            .ml_auto()
                            .gap(px(4.))
                            .items_center()
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Draft"),
                            ),
                    );
                    col = col.child(
                        v_flex()
                            .rounded(px(8.))
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .px(px(12.))
                            .py(px(10.))
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(title),
                            )
                            .child(meta),
                    );
                }
                kanban_cols = kanban_cols.child(col);
            }
        }
        let kanban_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .px(px(12.))
                    .pt(px(9.))
                    .pb(px(6.))
                    .gap(px(8.))
                    .items_center()
                    .child(
                        svg_icon(icons::SIDEBAR_LEFT)
                            .size(px(18.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Kanban"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(format!("{kanban_count} tasks")),
                    )
                    .child(div().flex_1())
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(soft_bg)
                            .child(seg_chip("seg-kb-list", "List", false).on_click(cx.listener(
                                |this, _e, _w, cx| this.open_route(1, cx),
                            )))
                            .child(seg_chip("seg-kb-kanban", "Kanban", true).on_click(
                                cx.listener(|this, _e, _w, cx| this.open_route(2, cx)),
                            )),
                    )
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(soft_bg)
                            .child(seg_chip("seg-kb-att", "Attention", true))
                            .child(seg_chip("seg-kb-classic", "Classic", false)),
                    )
                    .child(
                        div()
                            .h(px(26.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .child(div().text_size(px(13.)).child("+ New task")),
                    ),
            )
            .child(
                h_flex()
                    .px(px(16.))
                    .pt(px(12.))
                    .pb(px(8.))
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .size(px(12.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child("Needs review"),
                    ),
            )
            .child(kanban_cols);

        // /pull-requests — "Code review" list panel + empty detail pane.
        // Upstream empty list: centered Empty block then per-repo warning callouts.
        let pr_rows = v_flex()
            .px_2()
            .pt(px(50.))
            .gap(px(12.))
            .child(
                v_flex()
                    .w_full()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("No pull requests"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("and issues found"),
                    )
                    .child(
                        div()
                            .pt(px(4.))
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child("Nothing here in the"),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child("repositories of your projects."),
                    ),
            )
            .child(
                div()
                    .mt(px(35.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if dark_ui { rgb(0x6b5423) } else { rgb(0xf0d9a8) })
                    .bg(if dark_ui { rgb(0x2e2512) } else { rgb(0xfdf6e8) })
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(theme.foreground)
                    .child("1 project repository was unavailable. Healthy repositories are still shown."),
            );
        let pullreq_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Code review"),
                    )
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::SORT)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().w(px(8.)))
                    .child(
                        svg_icon(icons::FILTER)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().w(px(8.)))
                    .child(
                        svg_icon(icons::MORE)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                h_flex()
                    .mx_1()
                    .mb_2()
                    .h(px(30.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(popover_bg)
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Search or paste a PR link"),
                    ),
            )
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .gap_4()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .h(px(24.))
                            .px_2()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(soft_bg)
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .child("All"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("0"),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_baseline()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("PRs"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("0"),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_baseline()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("Issues"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("0"),
                            ),
                    ),
            )
            .child(pr_rows);
        let pullreq_pane = v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                svg_icon(icons::PULL_REQUESTS)
                    .size(px(36.))
                    .text_color(theme.muted_foreground),
            )
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .child("Select a pull request or issue"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Choose one from the sidebar to review it"),
            );

        // /automations — definition list panel + selected detail pane
        // (upstream RailAutomationsPanel + automation runs view).
        let mut autom_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                div()
                    .px_2()
                    .pb_2()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Automations"),
            )
            .child(
                h_flex()
                    .id("autom-new")
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(|this, _e, w, cx| {
                        this.open_autom_dialog(None, w, cx);
                    }))
                    .child(
                        svg_icon(icons::PLUS_MED)
                            .size(px(14.))
                            .text_color(theme.foreground),
                    )
                    .child(div().text_size(px(13.)).child("New automation")),
            );
        for def in self.autom_defs.clone() {
            let id = str_at(&def, "id").to_string();
            let sel = self.autom_sel.as_deref() == Some(id.as_str());
            let enabled = get(&def, "enabled").as_bool().unwrap_or(false);
            autom_panel = autom_panel.child(
                h_flex()
                    .id(ElementId::Name(format!("autom-{id}").into()))
                    .px_2()
                    .py_1p5()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(sel, |d| d.bg(theme.secondary))
                    .when(!sel, |d| d.hover(|d| d.bg(theme.secondary)))
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.autom_sel = Some(id.clone());
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .flex_shrink_0()
                            .bg(if enabled {
                                Hsla::from(rgb(0x34c759))
                            } else {
                                theme.muted_foreground
                            }),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .overflow_hidden()
                                    .child(str_at(&def, "name").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted_foreground)
                                    .child(Self::autom_summary(get(&def, "schedule"))),
                            ),
                    ),
            );
        }
        if self.autom_defs.is_empty() {
            autom_panel = autom_panel.child(
                div()
                    .mt(px(26.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("No automations yet"),
            );
        }
        let autom_sel_def = self
            .autom_sel
            .as_ref()
            .and_then(|sel| {
                self.autom_defs
                    .iter()
                    .find(|d| str_at(d, "id") == sel.as_str())
                    .cloned()
            });
        let mut autom_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .pt(px(9.))
                    .pr(px(19.))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("autom-refresh")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.fetch_automations(cx);
                            }))
                            .child(
                                svg_icon(icons::REFRESH)
                                    .size(px(18.))
                                    .text_color(theme.muted_foreground),
                            ),
                    ),
            );
        if let Some(def) = autom_sel_def.clone() {
            let def_id = str_at(&def, "id").to_string();
            let enabled = get(&def, "enabled").as_bool().unwrap_or(false);
            let proj_name = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "projects")
                        .iter()
                        .find(|p| str_at(p, "id") == str_at(&def, "projectId"))
                        .map(|p| str_at(p, "title").to_string())
                })
                .unwrap_or_default();
            let mut detail = v_flex().flex_1().px_4().gap_3().min_h_0();
            detail = detail.child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .overflow_hidden()
                            .child(str_at(&def, "name").to_string()),
                    )
                    .child(
                        div()
                            .px_2()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .bg(if enabled {
                                Hsla::from(rgb(0x34c759)).opacity(0.15)
                            } else {
                                theme.secondary
                            })
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if enabled {
                                        Hsla::from(rgb(0x34c759))
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child(if enabled { "Enabled" } else { "Disabled" }),
                            ),
                    ),
            );
            let meta_rows: Vec<(String, String)> = vec![
                ("Schedule".into(), Self::autom_summary(get(&def, "schedule"))),
                ("Project".into(), proj_name),
                (
                    "Next run".into(),
                    str_at(&def, "nextRunAt").to_string().replace('T', " ").trim_end_matches('Z').to_string(),
                ),
                (
                    "Mode".into(),
                    str_at(&def, "mode").to_string(),
                ),
            ];
            for (k, v) in meta_rows {
                if v.is_empty() {
                    continue;
                }
                detail = detail.child(
                    h_flex()
                        .text_size(px(12.))
                        .child(
                            div()
                                .w(px(90.))
                                .text_color(theme.muted_foreground)
                                .child(k),
                        )
                        .child(div().flex_1().min_w_0().child(v)),
                );
            }
            let prompt_txt = str_at(&def, "prompt").to_string();
            if !prompt_txt.is_empty() {
                detail = detail.child(
                    div()
                        .w_full()
                        .p_2()
                        .rounded(px(8.))
                        .bg(theme.secondary)
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(prompt_txt),
                );
            }
            let runs: Vec<Value> = self
                .autom_runs
                .iter()
                .filter(|r| str_at(r, "automationId") == def_id.as_str())
                .cloned()
                .collect();
            detail = detail.child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Runs"),
            );
            if runs.is_empty() {
                detail = detail.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("No runs yet"),
                );
            }
            let mut run_list = v_flex().gap_1();
            for run in runs.iter().take(10) {
                run_list = run_list.child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .child(
                            div()
                                .w(px(64.))
                                .text_color(theme.muted_foreground)
                                .child(str_at(run, "status").to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .child(
                                    str_at(run, "scheduledFor")
                                        .to_string()
                                        .replace('T', " ")
                                        .trim_end_matches('Z')
                                        .to_string(),
                                ),
                        )
                        .child(
                            div()
                                .text_color(theme.muted_foreground)
                                .child(
                                    get(run, "trigger")
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .map(str::to_string)
                                        .unwrap_or_else(|| str_at(run, "trigger").to_string()),
                                ),
                        ),
                );
            }
            detail = detail.child(run_list);
            let id_run = def_id.clone();
            let id_del = def_id.clone();
            let def_edit = def.clone();
            detail = detail.child(
                h_flex()
                    .mt_2()
                    .gap_2()
                    .child(
                        div()
                            .id("autom-runnow")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .bg(theme.foreground)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.background)
                                    .child("Run now"),
                            )
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.autom_run_now(id_run.clone(), cx);
                            })),
                    )
                    .child(
                        div()
                            .id("autom-edit")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(div().text_size(px(12.)).child("Edit"))
                            .on_click(cx.listener(move |this, _e, w, cx| {
                                this.open_autom_dialog(Some(def_edit.clone()), w, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("autom-delete")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(0xff453a))
                                    .child("Delete"),
                            )
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.autom_delete(id_del.clone(), cx);
                            })),
                    ),
            );
            autom_pane = autom_pane.child(detail);
        } else {
            autom_pane = autom_pane.child(
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .pb(px(120.))
                    .child(
                        svg_icon(icons::AUTOMATIONS)
                            .size(px(32.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Automations"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Pick an automation in the panel to see its runs, or"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("schedule a new one."),
                    )
                    .child(
                        div()
                            .id("autom-new2")
                            .mt_2()
                            .h(px(30.))
                            .px_4()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .bg(theme.foreground)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _e, w, cx| {
                                this.open_autom_dialog(None, w, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.background)
                                    .child("New automation"),
                            ),
                    ),
            );
        }

        // /hubs (studio rail icon) — hubs list panel + empty pane.
        let studio_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Hubs"),
                    )
                    .child(
                        div()
                            .pl_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("\u{2304}"),
                    )
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .child(
                        svg_icon(icons::PENCIL)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().text_size(px(13.)).child("New hub")),
            )
            .child(
                div()
                    .mt(px(14.))
                    .px_2()
                    .py_1()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Hubs"),
            )
            .child(
                div()
                    .mt(px(24.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("No hubs yet"),
            );
        let studio_pane = v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .child("No hubs yet"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Create one from the sidebar to coordinate work across repos."),
            );



        // ---- terminal surface (upstream workspace mode: the Terminal surface
        // replaces transcript + composer; TerminalWorkspaceTabs switches surfaces) ----
        let mut term_sub_strip = h_flex()
            .px(px(20.))
            .pt(px(6.))
            .gap(px(6.))
            .items_end()
            .border_b_1()
            .border_color(rgba(0x00000012));
        for (ti, tab) in self.term_tabs.iter().enumerate() {
            let is_sel = ti == self.term_sel;
            let mut chip = div()
                .id(SharedString::from(format!("term-tab-{ti}")))
                .h(px(28.))
                .mb(px(-1.))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .rounded_t(px(10.))
                .cursor_pointer()
                .font_family(theme.mono_font_family.clone())
                .text_size(px(13.))
                .when(is_sel, |d| {
                    d.border_1()
                        .border_b_0()
                        .border_color(rgba(0x00000012))
                        .bg(theme.background)
                })
                .when(!is_sel, |d| {
                    d.text_color(theme.muted_foreground)
                        .hover(|d| d.bg(subtle))
                })
                .child(format!("Terminal {}", ti + 1))
                .on_click(cx.listener(move |this, _ev, _w, cx| {
                    this.term_select_tab(ti, cx)
                }));
            if self.term_tabs.len() > 1 {
                chip = chip.child(
                    div()
                        .id(SharedString::from(format!("term-tab-x-{ti}")))
                        .size(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .hover(|d| d.bg(subtle))
                        .child("\u{00d7}")
                        .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx| {
                            this.term_close_tab(ti, cx)
                        })),
                );
            }
            term_sub_strip = term_sub_strip.child(chip);
        }
        term_sub_strip = term_sub_strip
            .child(
                div()
                    .id("term-new-tab")
                    .h(px(28.))
                    .mb(px(-1.))
                    .px_2()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("+")
                    .on_click(cx.listener(|this, _ev, _w, cx| this.term_new_tab(cx))),
            )
            .child(
                div()
                    .id("term-surface-chat")
                    .h(px(28.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_t(px(10.))
                    .cursor_pointer()
                    .font_family(theme.mono_font_family.clone())
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("Chat")
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.term_active = false;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("term-search-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child(svg_icon(icons::SEARCH).size(px(12.)))
                    .on_click(cx.listener(|this, _ev, w, cx| this.toggle_term_search(w, cx))),
            )
            .child(
                div()
                    .id("term-surface-close")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("\u{00d7}")
                    .on_click(cx.listener(|this, _ev, w, cx| this.toggle_terminal(w, cx))),
            );
        let mut term_surface = v_flex().flex_1().min_h_0().w_full().bg(content_bg);
        term_surface = term_surface
            .child(term_sub_strip)
            .when(self.term_search_input.is_some(), |d| {
                let inp = self.term_search_input.clone().unwrap();
                let q = self.term_search_query.to_lowercase();
                let matches = if q.is_empty() {
                    0
                } else {
                    self.term_tabs
                        .get(self.term_sel)
                        .map(|t| &t.buf[..])
                        .unwrap_or("")
                        .lines()
                        .filter(|l| l.to_lowercase().contains(&q))
                        .count()
                };
                d.child(
                    h_flex()
                        .px(px(13.))
                        .py(px(4.))
                        .gap_2()
                        .items_center()
                        .border_b_1()
                        .border_color(rgba(0x00000012))
                        .child(
                            div()
                                .w(px(200.))
                                .h(px(24.))
                                .px_2()
                                .flex()
                                .items_center()
                                .rounded(px(6.))
                                .border_1()
                                .border_color(theme.border)
                                .child(Input::new(&inp).appearance(false)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(if q.is_empty() {
                                    String::new()
                                } else {
                                    format!("{matches} match{}", if matches == 1 { "" } else { "es" })
                                }),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scrollbar()
                    .px(px(13.))
                    .py_2()
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .when(!self.term_search_query.is_empty(), |d| {
                        let q = self.term_search_query.to_lowercase();
                        let mut col = v_flex().w_full();
                        let buf: &str = self
                            .term_tabs
                            .get(self.term_sel)
                            .map(|t| &t.buf[..])
                            .unwrap_or("");
                        for line in buf.lines() {
                            let hit = line.to_lowercase().contains(&q);
                            col = col.child(
                                div()
                                    .w_full()
                                    .when(hit, |d| d.bg(theme.primary.opacity(0.14)))
                                    .child(line.to_string()),
                            );
                        }
                        d.child(col)
                    })
                    .when(self.term_search_query.is_empty(), |d| {
                        let buf = self
                            .term_tabs
                            .get(self.term_sel)
                            .map(|t| t.buf.clone())
                            .unwrap_or_default();
                        d.child(if buf.is_empty() {
                            "Starting terminal\u{2026}".to_string()
                        } else {
                            buf
                        })
                    })
                    .when_some(self.term_input.clone(), |d, inp| {
                        // The command line renders as the next terminal line —
                        // upstream has no separate input row.
                        d.child(
                            div()
                                .pt(px(4.))
                                .child(Input::new(&inp).appearance(false)),
                        )
                    }),
            );

        // ---- transcript find bar (Cmd+F) ----
        // Upstream ThreadFindBar: floating card at top-right (w-80 rounded-3xl
        // border bg-elevated shadow-lg). Row 1: search icon + input (h-11) +
        // divider + X close. Row 2 (border-t): prev/next arrows + match count.
        let mut find_bar = div().w(px(320.));
        if let Some(fi) = &self.find_input {
            let detail = self.detail.clone().unwrap_or(Value::Null);
            let tv = get(&detail, "thread");
            let q = self.find_query.to_lowercase();
            let count = if q.is_empty() {
                0
            } else {
                arr(tv, "messages")
                    .iter()
                    .filter(|m| {
                        let t = str_at(m, "text").to_string();
                        let t = if t.trim().is_empty() {
                            get(m, "textSegments")
                                .as_array()
                                .map(|segs| {
                                    segs.iter()
                                        .filter_map(|s| {
                                            s.get("text").and_then(Value::as_str)
                                        })
                                        .collect::<Vec<_>>()
                                        .join("")
                                })
                                .unwrap_or_default()
                        } else {
                            t
                        };
                        t.to_lowercase().contains(&q)
                    })
                    .count()
            };
            let glyph = theme.muted_foreground;
            find_bar = find_bar
                .rounded(px(24.))
                .border_1()
                .border_color(theme.border.opacity(0.6))
                .bg(popover_bg)
                .shadow_lg()
                .child(
                    h_flex()
                        .items_center()
                        .h(px(44.))
                        .px_4()
                        .gap(px(10.))
                        .child(
                            svg_icon(icons::SEARCH)
                                .size(px(16.))
                                .text_color(glyph),
                        )
                        .child(
                            div().flex_1().min_w_0().child(
                                Input::new(fi).appearance(false),
                            ),
                        )
                        .child(div().h(px(20.)).w(px(1.)).bg(theme.border))
                        .child(
                            Button::new("find-close")
                                .ghost()
                                .compact()
                                .icon(
                                    svg_icon(icons::X)
                                        .size(px(16.))
                                        .text_color(glyph),
                                )
                                .on_click(cx.listener(|this, _ev, w, cx| {
                                    this.toggle_find(w, cx);
                                })),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .border_t_1()
                        .border_color(theme.border.opacity(0.6))
                        .px_3()
                        .py_2()
                        .child(
                            h_flex().items_center().gap_1()
                                .child(
                                    Button::new("find-prev")
                                        .ghost()
                                        .compact()
                                        .icon(
                                            svg_icon(icons::SEND)
                                                .size(px(16.))
                                                .text_color(glyph),
                                        ),
                                )
                                .child(
                                    Button::new("find-next")
                                        .ghost()
                                        .compact()
                                        .icon(
                                            svg_icon(icons::ARROW_DOWN)
                                                .size(px(16.))
                                                .text_color(glyph),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(format!("{count} matches")),
                        ),
                );
        }

        // ---- main column under the strip: panes + composer ----
        // .relative() hosts the provider-update toast overlay.
        let mut main = v_flex().flex_1().h_full().relative();
        if self.view_settings {
            main = main.child(div().flex_1().child(settings_pane));
        } else if self.view_dev {
            main = main.child(div().flex_1().child(dev_pane.overflow_y_scrollbar()));
        } else if self.view_files {
            main = main.child(div().flex_1().child(files_pane.overflow_y_scrollbar()));
        } else if self.view_inbox {
            main = main.child(div().flex_1().child(inbox_pane.overflow_y_scrollbar()));
        } else if self.view_tasks {
            main = main.child(div().flex_1().child(tasks_pane));
        } else if self.view_kanban {
            main = main
                .child(div().flex_1().child(kanban_pane.overflow_y_scrollbar()));
        } else if self.view_pullreq {
            main = main.child(div().flex_1().child(pullreq_pane));
        } else if self.view_autom {
            main = main.child(div().flex_1().child(autom_pane));
        } else if self.view_studio {
            main = main.child(div().flex_1().child(studio_pane));
        } else {
            if self.view_terminal && self.term_active {
                main = main.child(term_surface);
            } else if self.split_open {
                // Zed-style split: two ratio-sized panes + draggable divider.
                let ratio = self.split_ratio.clamp(0.25, 0.75);
                let left = v_flex()
                    .id("split-left")
                    .w(gpui::relative(ratio))
                    .min_w_0()
                    .h_full()
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.focused_pane = 0;
                        cx.notify();
                    }))
                    .can_drop(|v, _w, _cx| {
                        v.is::<ThreadDrag>() || v.is::<TabDrag>()
                    })
                    .on_drop::<ThreadDrag>(cx.listener(
                        |this, drag: &ThreadDrag, _w, cx| {
                            this.focused_pane = 0;
                            this.open_thread(drag.0.clone(), cx);
                        },
                    ))
                    .on_drop::<TabDrag>(cx.listener(
                        |this, drag: &TabDrag, _w, cx| {
                            this.focused_pane = 0;
                            this.open_thread(drag.0.clone(), cx);
                        },
                    ))
                    .child(div().flex_1().child(
                        pane.id("transcript-scroll")
                            .track_scroll(&self.transcript_scroll)
                            .overflow_y_scroll()
                            .vertical_scrollbar(&self.transcript_scroll),
                    ))
                    .child(composer_bar);
                // 9px hit strip drawn as a centered 1px line over the
                // boundary (Zed divider affordance).
                let divider = div()
                    .id("split-divider")
                    .w(px(9.))
                    .h_full()
                    .ml(px(-4.))
                    .mr(px(-4.))
                    .cursor_col_resize()
                    .child(
                        div().w(px(1.)).h_full().ml(px(4.)).bg(theme.border),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.split_dragging = true;
                            cx.notify();
                        }),
                    );
                let right = v_flex()
                    .id("split-right")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .border_l_1()
                    .border_color(theme.border)
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.focused_pane = 1;
                        cx.notify();
                    }))
                    .child(div().flex_1().child(split_pane.overflow_y_scrollbar()))
                    .child(split_bar);
                let cols = h_flex()
                    .id("split-cols")
                    .flex_1()
                    .h_full()
                    .child(left)
                    .child(divider)
                    .child(right);
                main = main.child(cols);
                if self.find_open {
                    main = main.child(
                        div().absolute().top(px(8.)).right(px(8.)).child(find_bar),
                    );
                }
            } else {
                main = main.child(div().flex_1().child(
                    pane.id("transcript-scroll")
                        .track_scroll(&self.transcript_scroll)
                        .overflow_y_scroll()
                        .vertical_scrollbar(&self.transcript_scroll),
                ));
                main = main.child(composer_bar);
                // Floating scroll-to-bottom (upstream ChatTranscriptPane):
                // appears once the transcript is scrolled up, sits above the
                // composer centered in the chat column.
                if has_thread {
                    let off = self.transcript_scroll.offset();
                    let max = self.transcript_scroll.max_offset();
                    // offset.y is negative: 0 at the top, -max at the bottom.
                    if max.y + off.y > px(24.) {
                        main = main.child(
                            h_flex()
                                .w_full()
                                .absolute()
                                .bottom(px(84.))
                                .justify_center()
                                .child(
                                    div()
                                        .id("scroll-to-bottom")
                                        .size(px(32.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(popover_bg)
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .on_click(cx.listener(
                                            |this, _ev, _w, cx| {
                                                this.transcript_scroll
                                                    .scroll_to_bottom();
                                                cx.notify();
                                            },
                                        ))
                                        .child(
                                            svg_icon(icons::ARROW_DOWN)
                                                .size(px(14.))
                                                .text_color(theme.foreground),
                                        ),
                                ),
                        );
                    }
                }
                if self.find_open {
                    main = main.child(
                        div().absolute().top(px(8.)).right(px(8.)).child(find_bar),
                    );
                }
                // Inline revert confirmation (upstream native confirm dialog):
                // "Revert this thread to checkpoint N?"
                if let Some((_mid, tc)) = self.pending_revert.clone() {
                    main = main.child(
                        h_flex()
                            .w_full()
                            .absolute()
                            .bottom(px(140.))
                            .justify_center()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(popover_bg)
                                    .px_3()
                                    .py_2()
                                    .child(
                                        div().text_xs().child(format!(
                                            "Revert this thread to checkpoint {tc}? This discards newer messages and turn diffs."
                                        )),
                                    )
                                    .child(
                                        div()
                                            .id("revert-cancel")
                                            .cursor_pointer()
                                            .rounded_md()
                                            .px_2()
                                            .py_1()
                                            .text_xs()
                                            .hover(|d| d.bg(subtle))
                                            .on_click(cx.listener(
                                                |this, _ev, _w, cx| {
                                                    this.pending_revert = None;
                                                    cx.notify();
                                                },
                                            ))
                                            .child("Cancel"),
                                    )
                                    .child(
                                        div()
                                            .id("revert-confirm")
                                            .cursor_pointer()
                                            .rounded_md()
                                            .px_2()
                                            .py_1()
                                            .text_xs()
                                            .text_color(theme.primary_foreground)
                                            .bg(theme.primary)
                                            .on_click(cx.listener(
                                                move |this, _ev, _w, cx| {
                                                    this.pending_revert = None;
                                                    let Some(tid) = this
                                                        .selected_thread
                                                        .clone()
                                                    else {
                                                        return;
                                                    };
                                                    let cell =
                                                        this.client.clone();
                                                    cx.background_executor()
                                                        .spawn(async move {
                                                            let Some(client) =
                                                                ensure_client(&cell)
                                                            else {
                                                                return;
                                                            };
                                                            client.call(
                                                                "orchestration.dispatchCommand",
                                                                json!({
                                                                    "type": "thread.checkpoint.revert",
                                                                    "commandId": uuid(),
                                                                    "threadId": tid,
                                                                    "turnCount": tc,
                                                                    "scope": "thread",
                                                                    "createdAt": now_iso(),
                                                                }),
                                                            );
                                                        })
                                                        .detach();
                                                    cx.notify();
                                                },
                                            ))
                                            .child("Revert"),
                                    ),
                            ),
                    );
                }
            }
        }
        if let Some((msg, is_err)) = self.feedback_toast.clone() {
            toast_shown = true;
            update_toast = update_toast.child(
                h_flex()
                    .w(px(364.))
                    .gap_2()
                    .items_center()
                    .rounded(px(12.))
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .p(px(14.))
                    .child(
                        Icon::new(if is_err {
                            AssetIcon::TriangleAlert
                        } else {
                            AssetIcon::Check
                        })
                        .size(px(16.))
                        .text_color(if is_err {
                            let c: gpui::Hsla = rgb(0xc24904).into();
                            c
                        } else {
                            let c: gpui::Hsla = rgb(0x2da44e).into();
                            c
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(msg),
                    ),
            );
        }
        if toast_shown {
            main = main.child(update_toast);
        }

        // ---- changes dock (right pane): upstream GitPanel ----
        // Header "Source control" + refresh + close; sections Staged/Changes
        // with file rows (icon, dir muted + name, +ins/-del, stage action);
        // bottom diff viewport over a border-t.
        let mut changes_pane = v_flex().w(px(300.)).h_full().flex_col();
        changes_pane = changes_pane.child(
            h_flex()
                .items_center()
                .w_full()
                .px_4()
                .pt(px(9.))
                .pb_1()
                .gap(px(6.))
                // gpui scroll offsets go negative toward the end of content.
                .when(self.dock_tabs_scroll.offset().x < px(0.), |row| {
                    // Overflow chevron (upstream SurfaceContentTabs
                    // affordance) — scrolls the tab strip back to start.
                    row.child(
                        div()
                            .id("dock-tabs-left")
                            .px(px(1.))
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.dock_tabs_scroll
                                    .set_offset(gpui::point(px(0.), px(0.)));
                                cx.notify();
                            }))
                            .child(Icon::new(IconName::ChevronLeft).size(px(14.))),
                    )
                })
                .child(
                    // Upstream SurfaceContentTabs: icon + label per pane,
                    // the strip scrolls horizontally past its width floor.
                    div()
                        .id("dock-tabs")
                        .flex_1()
                        .min_w_0()
                        .track_scroll(&self.dock_tabs_scroll)
                        .overflow_x_scroll()
                        .child(
                            // Fixed natural width so Taffy reports real
                            // overflow for the scroll range (tabs ~430px).
                            h_flex()
                                .gap_1()
                                .w(px(430.))
                                .children(
                                    [
                                        (0usize, "dock-tab-git", "Git", icons::GIT_COMMIT),
                                        (1usize, "dock-tab-sub", "Subagents", icons::ROBOT),
                                        (2usize, "dock-tab-tasks", "Background tasks", icons::DOWN_WALL),
                                        (3usize, "dock-tab-side", "Side chats", icons::SIDECHAT),
                                    ]
                                    .into_iter()
                                    .map(|(tab, id, label, icon)| {
                                        div()
                                            .id(id)
                                            .px_2()
                                            .py(px(2.))
                                            .rounded(px(6.))
                                            .cursor_pointer()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .flex_none()
                                            .whitespace_nowrap()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .when(self.dock_tab == tab, |d| d.bg(subtle))
                                            .when(self.dock_tab != tab, |d| {
                                                d.text_color(theme.muted_foreground)
                                            })
                                            .child(
                                                svg_icon(icon)
                                                    .size(px(14.))
                                                    .text_color(
                                                        theme.muted_foreground,
                                                    ),
                                            )
                                            .child(label)
                                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                this.dock_tab = tab;
                                                cx.notify();
                                            }))
                                    }),
                                ),
                        ),
                )
                .when(
                    self.dock_tabs_scroll.offset().x > -self.dock_tabs_scroll.max_offset().x,
                    |row| {
                        row.child(
                            div()
                                .id("dock-tabs-right")
                                .px(px(1.))
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    let max = this.dock_tabs_scroll.max_offset();
                                    this.dock_tabs_scroll
                                        .set_offset(gpui::point(-max.x, px(0.)));
                                    cx.notify();
                                }))
                                .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.))),
                        )
                    },
                )
                .child(
                    Button::new("changes-refresh")
                        .ghost()
                        .compact()
                        .icon(IconName::RefreshCw)
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.changes_diff = None;
                            this.fetch_changes(cx);
                        })),
                )
                .child(
                    Button::new("changes-close")
                        .ghost()
                        .compact()
                        .icon(
                            svg_icon(icons::X)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.view_changes = false;
                            this.blame_pop = None;
                            cx.notify();
                        })),
                ),
        );
        // -- sections (top, capped height, scrollable) --
        let selected_path = self.changes_diff.as_ref().map(|(p, _)| p.clone());
        if self.dock_tab == 1 {
            // SubagentsDockPane: threads created as children of another thread
            // (parentThreadId set) — nickname/role when present.
            let mut sub_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("subagents-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            let subs: Vec<&Value> = arr(
                self.snapshot.as_ref().unwrap_or(&Value::Null),
                "threads",
            )
            .iter()
            .filter(|t| {
                !get(t, "parentThreadId").is_null()
                    && get(t, "deletedAt").is_null()
            })
            .collect();
            if subs.is_empty() {
                sub_area = sub_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No subagents yet"),
                );
            } else {
                for (si, t) in subs.iter().enumerate() {
                    let tid = str_at(t, "id").to_string();
                    let name = {
                        let n = str_at(t, "subagentNickname");
                        if n.is_empty() {
                            let ti = str_at(t, "title");
                            if ti.is_empty() { "Subagent".to_string() } else { ti.to_string() }
                        } else {
                            n.to_string()
                        }
                    };
                    let role = str_at(t, "subagentRole").to_string();
                    sub_area = sub_area.child(
                        div()
                            .id(SharedString::from(format!("subag-{si}")))
                            .w_full()
                            .px_2()
                            .py(px(5.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_thread(tid.clone(), cx)
                            }))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(name),
                                    )
                                    .when(!role.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(theme.muted_foreground)
                                                .child(role.clone()),
                                        )
                                    }),
                            ),
                    );
                }
            }
            changes_pane = changes_pane.child(sub_area);
        } else if self.dock_tab == 2 {
            // BackgroundTasksDockPane: the thread's background tasks, derived
            // from its activities like upstream's workLog. Running rows show a
            // Stop action dispatching thread.task.stop.
            let thread_v = get(self.detail.as_ref().unwrap_or(&Value::Null), "thread").clone();
            let tasks = derive_bg_tasks(&thread_v);
            let mut task_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("tasks-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            if tasks.is_empty() {
                task_area = task_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No background tasks in this chat yet"),
                );
            } else {
                let running: Vec<&BgTask> =
                    tasks.iter().filter(|t| t.status == "running").rev().collect();
                let finished: Vec<&BgTask> =
                    tasks.iter().filter(|t| t.status != "running").rev().collect();
                let row = |t: &BgTask, on_stop: bool, this_tid: &str| {
                    let status_label = match t.status.as_str() {
                        "running" => "running".to_string(),
                        "finished" => "finished".to_string(),
                        "failed" => match t.exit_code {
                            Some(c) => format!("failed · exit {c}"),
                            None => "failed".to_string(),
                        },
                        "stopped" => "stopped".to_string(),
                        _ => t.status.clone(),
                    };
                    let when = rel_time(t.completed_at.as_deref().unwrap_or(t.started_at.as_str()));
                    let stop_tid = t.task_id.clone();
                    let stop_thread = this_tid.to_string();
                    div()
                        .id(SharedString::from(format!("bgtask-{}", t.task_id)))
                        .w_full()
                        .px_2()
                        .py(px(5.))
                        .rounded(px(6.))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    svg_icon(icons::TERMINAL)
                                        .size(px(14.))
                                        .text_color(if t.status == "running" {
                                            theme.foreground
                                        } else {
                                            theme.muted_foreground
                                        }),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .child(t.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(if t.status == "failed" {
                                                    theme.danger
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .child(status_label),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(when),
                                )
                                .when(on_stop, |d| {
                                    d.child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "bgtask-stop-{}",
                                                t.task_id
                                            )))
                                            .px_1()
                                            .rounded(px(4.))
                                            .cursor_pointer()
                                            .hover(|h| h.bg(theme.border))
                                            .child(
                                                svg_icon(icons::X)
                                                    .size(px(12.))
                                                    .text_color(theme.muted_foreground),
                                            )
                                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                this.dispatch_cmd(
                                                    json!({
                                                        "type": "thread.task.stop",
                                                        "commandId": uuid(),
                                                        "threadId": stop_thread,
                                                        "taskId": stop_tid,
                                                        "createdAt": chrono::Utc::now()
                                                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                                                    }),
                                                    cx,
                                                );
                                            })),
                                    )
                                }),
                        )
                        .into_any_element()
                };
                let tid = self.selected_thread.clone().unwrap_or_default();
                if !running.is_empty() {
                    task_area = task_area.child(
                        div()
                            .px(px(8.))
                            .pt(px(4.))
                            .pb(px(2.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(format!("Active · {}", running.len())),
                    );
                    for t in running {
                        task_area = task_area.child(row(t, true, &tid));
                    }
                }
                if !finished.is_empty() {
                    task_area = task_area.child(
                        div()
                            .px(px(8.))
                            .pt(px(4.))
                            .pb(px(2.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(format!("Done · {}", finished.len())),
                    );
                    for t in finished.into_iter().take(10) {
                        task_area = task_area.child(row(t, false, &tid));
                    }
                }
            }
            changes_pane = changes_pane.child(task_area);
        } else if self.dock_tab == 3 {
            // SidechatDockPane: side chats forked from this thread
            // (sidechatSourceThreadId), plus a "New side chat" action that
            // dispatches thread.fork.create like upstream's sidechatCreation.
            let cur_tid = self.selected_thread.clone().unwrap_or_default();
            let mut side_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("sidechat-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            side_area = side_area.child(
                div()
                    .id("sidechat-new")
                    .w_full()
                    .px_2()
                    .py(px(5.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(subtle))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        let detail = this.detail.clone().unwrap_or(Value::Null);
                        let tv = get(&detail, "thread");
                        let Some(src) = this.selected_thread.clone() else { return };
                        let title_seed = str_at(tv, "title");
                        let imported: Vec<Value> = arr(tv, "messages")
                            .iter()
                            .filter(|m| {
                                !str_at(m, "text").is_empty()
                                    && get(m, "streaming").as_bool() != Some(true)
                            })
                            .map(|m| {
                                json!({
                                    "messageId": uuid(),
                                    "role": str_at(m, "role"),
                                    "text": str_at(m, "text"),
                                    "createdAt": str_at(m, "createdAt"),
                                    "updatedAt": str_at(m, "updatedAt"),
                                })
                            })
                            .collect();
                        this.dispatch_cmd(
                            json!({
                                "type": "thread.fork.create",
                                "commandId": uuid(),
                                "threadId": uuid(),
                                "sourceThreadId": src,
                                "sidechatSourceThreadId": src,
                                "projectId": str_at(tv, "projectId"),
                                "title": format!("Sidechat: {title_seed}"),
                                "modelSelection": get(tv, "modelSelection"),
                                "runtimeMode": str_at(tv, "runtimeMode"),
                                "interactionMode": "default",
                                "envMode": str_at(tv, "envMode"),
                                "branch": get(tv, "branch"),
                                "worktreePath": get(tv, "worktreePath"),
                                "workingDirectory": get(tv, "workingDirectory"),
                                "associatedWorktreePath": get(tv, "associatedWorktreePath"),
                                "associatedWorktreeBranch": get(tv, "associatedWorktreeBranch"),
                                "associatedWorktreeRef": get(tv, "associatedWorktreeRef"),
                                "importedMessages": imported,
                                "createdAt": chrono::Utc::now()
                                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                            }),
                            cx,
                        );
                    }))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                svg_icon(icons::PLUS)
                                    .size(px(13.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(div().text_size(px(13.)).child("New side chat")),
                    ),
            );
            let sides: Vec<&Value> = arr(
                self.snapshot.as_ref().unwrap_or(&Value::Null),
                "threads",
            )
            .iter()
            .filter(|t| {
                str_at(t, "sidechatSourceThreadId") == cur_tid
                    && get(t, "deletedAt").is_null()
            })
            .collect();
            if sides.is_empty() {
                side_area = side_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No side chats yet"),
                );
            } else {
                for (si, t) in sides.iter().enumerate() {
                    let tid = str_at(t, "id").to_string();
                    let title = {
                        let s = str_at(t, "title");
                        if s.is_empty() { "Side chat".to_string() } else { s.to_string() }
                    };
                    let when = rel_time(str_at(t, "updatedAt"));
                    side_area = side_area.child(
                        div()
                            .id(SharedString::from(format!("sidechat-{si}")))
                            .w_full()
                            .px_2()
                            .py(px(5.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_thread(tid.clone(), cx)
                            }))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child(when),
                                    ),
                            ),
                    );
                }
            }
            changes_pane = changes_pane.child(side_area);
        } else {
        let mut files_area = v_flex()
            .w_full()
            .flex_none()
            .max_h(px(430.))
            .id("changes-files-scroll")
            .overflow_y_scrollbar()
            .px(px(6.))
            .py_2()
            .gap_2();
        if self.changes_loading {
            files_area = files_area.child(
                div()
                    .px(px(6.))
                    .py_1()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground.opacity(0.7))
                    .child("Loading changes..."),
            );
        } else if self.changes_unavailable {
            files_area = files_area.child(
                div()
                    .w_full()
                    .py_2()
                    .flex()
                    .justify_center()
                    .text_size(px(14.))
                    .text_color(theme.muted_foreground.opacity(0.7))
                    .child("Source control is unavailable for this thread."),
            );
        } else if self.staged_files.is_empty() && self.unstaged_files.is_empty() {
            files_area = files_area.child(
                div()
                    .w_full()
                    .py_2()
                    .flex()
                    .justify_center()
                    .text_size(px(14.))
                    .text_color(theme.muted_foreground.opacity(0.7))
                    .child("No changes in the working tree."),
            );
        } else {
            for (sec_title, sec_files, empty_label, unstage, action_all) in [
                (
                    "Staged",
                    self.staged_files.clone(),
                    "No staged changes.",
                    true,
                    "Unstage all",
                ),
                (
                    "Changes",
                    self.unstaged_files.clone(),
                    "No unstaged changes.",
                    false,
                    "Stage all",
                ),
            ] {
                let adds: i64 = sec_files
                    .iter()
                    .map(|f| get(f, "insertions").as_i64().unwrap_or(0))
                    .sum();
                let dels: i64 = sec_files
                    .iter()
                    .map(|f| get(f, "deletions").as_i64().unwrap_or(0))
                    .sum();
                let all_paths: Vec<String> = sec_files
                    .iter()
                    .map(|f| str_at(f, "path").to_string())
                    .collect();
                let mut header = h_flex()
                    .items_center()
                    .gap_2()
                    .px(px(6.))
                    .py_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(sec_title),
                    )
                    .child(
                        div()
                            .rounded_full()
                            .bg(theme.muted)
                            .px(px(6.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(sec_files.len().to_string()),
                    )
                    .child(
                        div().text_size(px(11.)).text_color(theme.success).child(
                            format!("+{adds}"),
                        ),
                    )
                    .child(
                        div().text_size(px(11.)).text_color(theme.danger).child(
                            format!("-{dels}"),
                        ),
                    )
                    .child(div().flex_1());
                if !sec_files.is_empty() {
                    header = header.child(
                        Button::new(ElementId::Name(
                            format!("chg-{sec_title}-all").into(),
                        ))
                        .ghost()
                        .compact()
                        .label(action_all)
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.stage_paths(all_paths.clone(), unstage, cx);
                        })),
                    );
                }
                let mut section = v_flex().w_full().child(header);
                if sec_files.is_empty() {
                    section = section.child(
                        div()
                            .px(px(6.))
                            .py_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(empty_label),
                    );
                } else {
                    let mut rows = v_flex().gap(px(2.));
                    for f in sec_files {
                        let path = str_at(&f, "path").to_string();
                        let ins = get(&f, "insertions").as_i64().unwrap_or(0);
                        let del = get(&f, "deletions").as_i64().unwrap_or(0);
                        let (dir, name) = match path.rfind('/') {
                            Some(i) => {
                                (format!("{}/", &path[..i]), path[i + 1..].to_string())
                            }
                            None => (String::new(), path.clone()),
                        };
                        let p = path.clone();
                        let is_sel = selected_path.as_deref() == Some(path.as_str());
                        let mut row = h_flex()
                            .id(ElementId::Name(format!("chg-{path}").into()))
                            .w_full()
                            .gap(px(6.))
                            .items_center()
                            .rounded(px(6.))
                            .px(px(6.))
                            .py(px(4.))
                            .cursor_pointer();
                        if is_sel {
                            row = row.bg(theme.sidebar_accent);
                        } else {
                            row = row
                                .hover(|s| s.bg(theme.sidebar_accent.opacity(0.6)));
                        }
                        row = row
                            .child(
                                svg_icon(icons::FILE_TEXT)
                                    .size(px(16.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(14.))
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(
                                        h_flex()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_color(
                                                        theme.muted_foreground
                                                            .opacity(0.7),
                                                    )
                                                    .child(dir),
                                            )
                                            .child(div().child(name)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.success)
                                    .child(format!("+{ins}")),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.danger)
                                    .child(format!("-{del}")),
                            );
                        let p2 = path.clone();
                        let action_icon = if unstage {
                            icons::UNSTAGE
                        } else {
                            icons::STAGE
                        };
                        row = row
                            .child(
                                Button::new(ElementId::Name(
                                    format!("chg-act-{path}").into(),
                                ))
                                .ghost()
                                .compact()
                                .icon(
                                    svg_icon(action_icon)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.stage_paths(vec![p.clone()], unstage, cx);
                                })),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_diff(p2.clone(), cx);
                            }));
                        rows = rows.child(row);
                    }
                    section = section.child(rows);
                }
                files_area = files_area.child(section);
            }
        }
        changes_pane = changes_pane.child(files_area);
        // -- diff viewport over a border-t --
        let mut viewport = v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .border_t_1()
            .border_color(theme.border.opacity(0.7));
        if let Some((dpath, patch)) = self.changes_diff.clone() {
            let mut diff_view = v_flex().gap_0().px_2().py_2();
            let mut old_n: u32 = 0;
            let mut new_n: u32 = 0;
            for line in patch.lines().take(400) {
                let color = if line.starts_with('+') && !line.starts_with("+++") {
                    theme.success
                } else if line.starts_with('-') && !line.starts_with("---") {
                    theme.danger
                } else if line.starts_with("@@") {
                    theme.accent
                } else {
                    theme.muted_foreground
                };
                // Hunk headers reset the blame-side line counters;
                // upstream blames deleted lines in the base tree.
                let mut blame: Option<(u32, bool)> = None;
                if line.starts_with("@@") {
                    if let Some(h) = line.split_whitespace().nth(1) {
                        let mut it = h.split(',');
                        if let Some(a) = it.next().and_then(|v| v.trim_start_matches('-').parse::<u32>().ok()) {
                            old_n = a.saturating_sub(1);
                        }
                    }
                    if let Some(h) = line.split_whitespace().nth(2) {
                        let mut it = h.split(',');
                        if let Some(a) = it.next().and_then(|v| v.trim_start_matches('+').parse::<u32>().ok()) {
                            new_n = a.saturating_sub(1);
                        }
                    }
                } else if line.starts_with('+') && !line.starts_with("+++") {
                    new_n += 1;
                    blame = Some((new_n, false));
                } else if line.starts_with('-') && !line.starts_with("---") {
                    old_n += 1;
                    blame = Some((old_n, true));
                } else if line.starts_with(' ') {
                    old_n += 1;
                    new_n += 1;
                    blame = Some((new_n, false));
                }
                let row = div()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(color)
                    .child(line.to_string());
                if let Some((ln, base)) = blame {
                    let pth = dpath.clone();
                    diff_view = diff_view.child(
                        row.id(ElementId::Name(
                            format!("dline-{}-{}-{}", if base { 'o' } else { 'n' }, ln, line.len()).into(),
                        ))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, ev: &gpui::ClickEvent, _w, cx| {
                            this.fetch_blame(
                                pth.clone(),
                                ln,
                                base,
                                ev.position().x.as_f32(),
                                ev.position().y.as_f32(),
                                cx,
                            );
                        })),
                    );
                } else {
                    diff_view = diff_view.child(row);
                }
            }
            viewport = viewport.child(
                div()
                    .flex_1()
                    .id("diff-scroll")
                    .overflow_y_scrollbar()
                    .child(diff_view),
            );
        } else {
            viewport = viewport.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .text_size(px(14.))
                    .text_color(theme.muted_foreground.opacity(0.7))
                    .child("Select a file to view its diff."),
            );
        }
        changes_pane = changes_pane.child(viewport);
        }

        // ---- icon rail ----
        // --app-rail-width: 51px; 36px buttons on 8px radius, 6px gap,
        // 10px top/bottom padding. Idle glyphs sit at muted-foreground/58;
        // the active button gets the sidebar-selected (black/3%) tint and
        // an ink glyph. h-px w-5 hairlines separate the icon groups.
        let mut glyph_idle = theme.muted_foreground;
        glyph_idle.a *= 0.58;
        let rail_btn = |id: &'static str, icon: &'static [u8], active: bool, _cx: &mut Context<Self>| {
            div()
                .id(ElementId::Name(id.into()))
                .size(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .cursor_pointer()
                .when(active, |d| d.bg(theme.secondary))
                .when(!active, |d| {
                    d.hover(|h| h.bg(theme.secondary))
                })
                .child(
                    svg_icon(icon)
                        .size(px(20.))
                        .text_color(if active { theme.foreground } else { glyph_idle }),
                )
        };
        let _rail_divider = || {
            div()
                .w(px(20.))
                .h(px(1.))
                .bg(theme.border)
                .my(px(3.))
        };
        let home_active = !self.view_files
            && !self.view_dev
            && !self.view_settings
            && !self.view_inbox
            && !self.view_tasks
            && !self.view_kanban
            && !self.view_pullreq
            && !self.view_autom
            && !self.view_studio
            && !self.view_spaces;
        let rail = v_flex()
            .w(px(51.))
            .h_full()
            .pt(px(7.))
            .pb(px(4.))
            .items_center()
            .gap(px(6.))
            .child(
                rail_btn("nav-home", icons::HOME, home_active, cx).on_click(cx.listener(
                    |this, _ev, _w, cx| {
                        this.clear_views();
                        this.view_changes = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                rail_btn("nav-inbox", icons::INBOX, self.view_inbox, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.dismiss_coachmark("activity", cx);
                        this.toggle_inbox(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-spaces", icons::SPACES, self.view_spaces, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.open_route(6, cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-tasks", icons::TASKS, self.view_tasks, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.dismiss_coachmark("tasks", cx);
                        this.open_route(1, cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-pullrequests", icons::PULL_REQUESTS, self.view_pullreq, cx)
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.open_route(3, cx);
                    })),
            )
            .child(
                rail_btn("nav-automations", icons::AUTOMATIONS, self.view_autom, cx)
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.open_route(4, cx);
                    })),
            )
            .child(
                rail_btn("nav-studio", icons::STUDIO, self.view_studio, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.open_route(5, cx);
                    }),
                ),
            )
            .child({
                // Upstream "…" opens a popover: Hubs, then checkbox groups for
                // spaces and projects pinned in the rail, then Customize.
                let more_weak = cx.weak_entity();
                let snap = self.snapshot.clone().unwrap_or(Value::Null);
                let mut more_spaces: Vec<(String, String)> = Vec::new();
                more_spaces.push(("space:void".to_string(), "Void".to_string()));
                for s in arr(&snap, "spaces") {
                    let id = str_at(s, "id").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let name = str_at(s, "name").to_string();
                    more_spaces.push((format!("space:{id}"), name));
                }
                let mut more_projects: Vec<(String, String)> = Vec::new();
                for p in arr(&snap, "projects") {
                    // Upstream lists ordinary space projects only
                    // (isOrdinarySpaceProject): no chat/group/studio kinds.
                    if str_at(p, "kind") != "project" {
                        continue;
                    }
                    let id = str_at(p, "id").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let title = str_at(p, "title").to_string();
                    more_projects.push((format!("project:{id}"), title));
                }
                let pinned: Vec<String> = arr(
                    get(&self.settings_data, "settings"),
                    "railShortcuts",
                )
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
                Button::new("nav-more")
                    .ghost()
                    .compact()
                    .icon(svg_icon(icons::MORE).size(px(20.)).text_color(glyph_idle))
                    .dropdown_menu(move |menu, _, _| {
                        let mut m = menu
                            .min_w(px(256.))
                            .max_h(px(500.))
                            .item(PopupMenuItem::new("Hubs").on_click({
                                let weak = more_weak.clone();
                                move |_, _w, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.open_route(5, cx);
                                    });
                                }
                            }))
                            .item(PopupMenuItem::separator())
                            .item(PopupMenuItem::label("Spaces in the rail"));
                        for (key, label) in &more_spaces {
                            let checked = pinned.iter().any(|k| k == key);
                            let weak = more_weak.clone();
                            let key = key.clone();
                            m = m.item(
                                PopupMenuItem::new(label.clone())
                                    .checked(checked)
                                    .on_click(move |_, _w, cx| {
                                        let _ = weak.update(cx, |this, cx| {
                                            this.toggle_rail_shortcut(&key, cx);
                                        });
                                    }),
                            );
                        }
                        m = m
                            .item(PopupMenuItem::separator())
                            .item(PopupMenuItem::label("Projects in the rail"));
                        if more_projects.is_empty() {
                            m = m.item(
                                PopupMenuItem::new("No projects").disabled(true),
                            );
                        } else {
                            for (key, label) in &more_projects {
                                let checked = pinned.iter().any(|k| k == key);
                                let weak = more_weak.clone();
                                let key = key.clone();
                                m = m.item(
                                    PopupMenuItem::new(label.clone())
                                        .checked(checked)
                                        .on_click(move |_, _w, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.toggle_rail_shortcut(&key, cx);
                                            });
                                        }),
                                );
                            }
                        }
                        m.item(PopupMenuItem::separator())
                            .item(PopupMenuItem::new("Customize").on_click({
                                let weak = more_weak.clone();
                                move |_, _w, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.clear_views();
                                        this.view_settings = true;
                                        if this.settings_data.is_null() {
                                            this.fetch_settings(cx);
                                        }
                                        cx.notify();
                                    });
                                }
                            }))
                    })
            })
            .child(div().flex_1())
            .child({
                // Upstream help menu (SidebarHelpMenu): "What's new" header,
                // the three latest release rows (title + date), "Full
                // changelog", then Keybindings / Send feedback / Docs.
                // Release rows open the release-history dialog upstream; the
                // shell has no dialog surface yet, so they open the docs
                // page instead.
                let help_weak = cx.weak_entity();
                const RELEASES: [(&str, &str); 3] = [
                    ("Use connectors with Antigravity", "Nov 20"),
                    ("See what your agent is doing", "Nov 19"),
                    ("Play local videos", "Nov 17"),
                ];
                Button::new("nav-help")
                    .ghost()
                    .compact()
                    .icon(svg_icon(icons::HELP).size(px(20.)).text_color(glyph_idle))
                    .dropdown_menu(move |menu, _, _| {
                        let mut m = menu
                            .min_w(px(256.))
                            .item(PopupMenuItem::label("What's new"));
                        for (title, date) in RELEASES {
                            m = m.item(
                                PopupMenuItem::element(move |_, _| {
                                    h_flex()
                                        .w(px(230.))
                                        .justify_between()
                                        .child(div().text_size(px(13.)).child(title))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgba(0x777777aa))
                                                .child(date),
                                        )
                                })
                                .on_click(|_, _w, cx| {
                                    cx.open_url("https://trysynara.com/docs");
                                }),
                            );
                        }
                        m = m
                            .item(
                                PopupMenuItem::new("Full changelog")
                                    .icon(svg_icon(icons::STAR).size(px(14.)))
                                    .on_click(|_, _w, cx| {
                                        cx.open_url("https://trysynara.com/docs");
                                    }),
                            )
                            .item(PopupMenuItem::separator())
                            .item(
                                PopupMenuItem::new("Keybindings")
                                    .icon(svg_icon(icons::S_SHORTCUT).size(px(14.)))
                                    .on_click({
                                        let weak = help_weak.clone();
                                        move |_, _w, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.clear_views();
                                                this.view_settings = true;
                                                this.settings_section = "shortcuts";
                                                if this.settings_data.is_null() {
                                                    this.fetch_settings(cx);
                                                }
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .item(
                                PopupMenuItem::new("Send feedback")
                                    .icon(svg_icon(icons::PENCIL).size(px(14.)))
                                    .on_click(|_, _w, cx| {
                                        cx.open_url(
                                            "https://github.com/Emanuele-web04/synara/issues",
                                        );
                                    }),
                            );
                        m.item(
                            PopupMenuItem::new("Docs")
                                .icon(svg_icon(icons::FILE_TEXT).size(px(14.)))
                                .on_click(|_, _w, cx| {
                                    cx.open_url("https://trysynara.com/docs");
                                }),
                        )
                    })
            })
            .child({
                // Upstream signed-out account menu (AccountFooterControl):
                // "You're in local mode" header + Sign in button, then
                // Settings. Sign in opens the sign-in dialog upstream; the
                // shell's equivalent surface is Settings → Accounts.
                let user_weak = cx.weak_entity();
                Button::new("nav-user")
                    .ghost()
                    .compact()
                    .icon(svg_icon(icons::USER).size(px(20.)).text_color(glyph_idle))
                    .dropdown_menu(move |menu, _, _| {
                        let weak = user_weak.clone();
                        menu.min_w(px(256.))
                            .item(
                                PopupMenuItem::element(|_, _| {
                                    v_flex()
                                        .w(px(230.))
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .child("You're in local mode"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(rgba(0x777777aa))
                                                .child(
                                                    "Sign in to sync your profile and workspace across devices.",
                                                ),
                                        )
                                })
                                .disabled(true),
                            )
                            .item(
                                PopupMenuItem::element(|_, _| {
                                    div()
                                        .w(px(230.))
                                        .h(px(30.))
                                        .rounded(px(8.))
                                        .bg(rgb(0x1a1a1a))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(rgb(0xffffff))
                                                .child("Sign in"),
                                        )
                                })
                                .on_click({
                                    let w = weak.clone();
                                    move |_, _w, cx| {
                                        let _ = w.update(cx, |this, cx| {
                                            this.clear_views();
                                            this.view_settings = true;
                                            this.settings_section = "accounts";
                                            if this.settings_data.is_null() {
                                                this.fetch_settings(cx);
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                            )
                            .item(PopupMenuItem::separator())
                            .item(
                                PopupMenuItem::element(|_, _| {
                                    h_flex()
                                        .w(px(230.))
                                        .items_center()
                                        .justify_between()
                                        .child(div().text_size(px(13.)).child("Settings"))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgba(0x777777aa))
                                                .child("\u{2318},"),
                                        )
                                })
                                .on_click(move |_, _w, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        this.clear_views();
                                        this.view_settings = true;
                                        if this.settings_data.is_null() {
                                            this.fetch_settings(cx);
                                        }
                                        cx.notify();
                                    });
                                }),
                            )
                    })
            });

        // ---- top strip (AppShellTopStrip) ----
        // 47px #f7f7f7 band, borderless, over rail+panel+content: sidebar
        // toggle centered over the rail at left, Environment + dock toggles
        // at right (Hugeicons panel-top-open / layout-right).
        let strip = h_flex()
            .w_full()
            .h(px(47.))
            .bg(well_bg)
            .items_center()
            .pr(px(19.))
            .child(
                div()
                    .w(px(51.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        rail_btn("nav-sidebartoggle", icons::SIDEBAR_LEFT, false, cx).on_click(
                            cx.listener(|this, _ev, _w, cx| {
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            // Sidebar well (rail + panel) keeps the left of the strip clear;
            // open-thread tabs start where the content pane starts.
            .child(
                div()
                    .w(if self.sidebar_collapsed { px(0.) } else { px(257.) })
                    .flex_none(),
            )
            .child({
                // Open-thread tabs live inside the top strip (upstream ChatHeader
                // threadTabs): bordered pill with icon + title + x per open tab,
                // then a "+" that opens a new thread.
                let mut tabs = h_flex().gap_1().items_center();
                let n_tabs = self.open_threads.len();
                if !self.open_threads.is_empty() {
                    let snap_threads =
                        arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads");
                    let indicator = || {
                        div()
                            .w(px(2.))
                            .h(px(18.))
                            .rounded_full()
                            .bg(theme.accent)
                    };
                    for (ti, ot) in self.open_threads.clone().into_iter().enumerate() {
                        if self.tab_drop_target == Some(ti) {
                            tabs = tabs.child(indicator());
                        }
                        let active =
                            self.selected_thread.as_deref() == Some(ot.as_str());
                        let ot_title = snap_threads
                            .iter()
                            .find(|t| str_at(t, "id") == ot)
                            .map(|t| str_at(t, "title").to_string())
                            .filter(|t| !t.is_empty())
                            .unwrap_or_else(|| ot.chars().take(8).collect());
                        let ghost_title = ot_title.clone();
                        let sel = ot.clone();
                        let close = ot.clone();
                        let mut pill = h_flex()
                            .id(ElementId::Name(format!("strip-tab-{ot}").into()))
                            .h(px(28.))
                            .items_center()
                            .gap(px(6.))
                            .pl(px(9.))
                            .pr(px(6.))
                            .rounded(px(8.))
                            .cursor_pointer()
                            .border_1()
                            .bg(if active {
                                content_bg
                            } else {
                                gpui::transparent_black()
                            })
                            .border_color(if active {
                                theme.border
                            } else {
                                theme.border.opacity(0.)
                            })
                            .child(
                                svg_icon(provider_icon(str_at(
                                    snap_threads
                                        .iter()
                                        .find(|t| str_at(t, "id") == ot)
                                        .map(|t| get(t, "session"))
                                        .unwrap_or(&Value::Null),
                                    "providerName",
                                )))
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .whitespace_nowrap()
                                    .child(ot_title),
                            )
                            .child(
                                div()
                                    .id(ElementId::Name(
                                        format!("strip-tab-close-{close}").into(),
                                    ))
                                    .size(px(16.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(subtle))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child("\u{00d7}"),
                                    )
                                    .on_click(cx.listener(
                                        move |this, _ev, _w, cx| {
                                            this.close_tab(close.clone(), cx);
                                        },
                                    )),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if this.selected_thread.as_deref() != Some(sel.as_str()) {
                                    this.open_thread(sel.clone(), cx);
                                }
                            }))
                            // Drag the pill to reorder open-thread tabs; the
                            // half of the pill under the pointer picks the
                            // insertion side.
                            .on_drag(
                                TabDrag(ot.clone()),
                                move |_d, _pos, _w, cx| {
                                    cx.new(|_| DragGhost(ghost_title.clone()))
                                },
                            )
                            .on_drag_move::<TabDrag>(cx.listener(
                                move |this, ev: &gpui::DragMoveEvent<TabDrag>, _w, cx| {
                                    let mid = ev.bounds.origin.x
                                        + ev.bounds.size.width / 2.;
                                    let idx = if ev.event.position.x < mid {
                                        ti
                                    } else {
                                        ti + 1
                                    };
                                    if this.tab_drop_target != Some(idx) {
                                        this.tab_drop_target = Some(idx);
                                        cx.notify();
                                    }
                                },
                            ));
                        if !active {
                            pill = pill.hover(|d| d.bg(subtle));
                        }
                        tabs = tabs.child(pill);
                    }
                    if self.tab_drop_target == Some(n_tabs) {
                        tabs = tabs.child(indicator());
                    }
                }
                tabs = tabs.on_drop::<TabDrag>(cx.listener(
                    |this, drag: &TabDrag, _w, cx| {
                        let to = this
                            .tab_drop_target
                            .unwrap_or_else(|| this.open_threads.len());
                        if let Some(from) =
                            this.open_threads.iter().position(|t| t == &drag.0)
                        {
                            let id = this.open_threads.remove(from);
                            let to = if to > from { to - 1 } else { to }
                                .min(this.open_threads.len());
                            this.open_threads.insert(to, id);
                        }
                        this.tab_drop_target = None;
                        cx.notify();
                    },
                ));
                tabs = tabs.child({
                    // Upstream "+" opens a menu with New chat / New terminal.
                    let weak_chat = cx.weak_entity();
                    let weak_term = cx.weak_entity();
                    Button::new("nav-newtab")
                        .ghost()
                        .compact()
                        .icon(svg_icon(icons::PLUS))
                        .dropdown_menu(move |menu, _, _| {
                            menu.item(PopupMenuItem::new("New chat").on_click({
                                let weak = weak_chat.clone();
                                move |_, _w, cx| {
                                    let _ = weak.update(cx, |this, cx| this.new_thread(cx));
                                }
                            }))
                            .item(PopupMenuItem::new("New terminal").on_click({
                                let weak = weak_term.clone();
                                move |_, w, cx| {
                                    let _ = weak.update(cx, |this, cx| {
                                        if this.view_terminal {
                                            this.term_active = true;
                                            cx.notify();
                                        } else {
                                            this.toggle_terminal(w, cx);
                                        }
                                    });
                                }
                            }))
                        })
                });
                tabs
            })
            .child(div().flex_1())
            .child(
                rail_btn("nav-handoff", icons::HANDOFF, false, cx),
            )
            .child({
                // Upstream GitActionsControl: git quick-action menu. Commit
                // items open the commit dialog (AI-drafted messages need a
                // provider runtime); Push/Create PR map to the dialogs too.
                let weak = cx.weak_entity();
                let gs = self.git_status.clone().unwrap_or(Value::Null);
                let has_repo = self.git_cwd().is_some();
                let dirty = get(&gs, "hasWorkingTreeChanges")
                    .as_bool()
                    .unwrap_or(false);
                let has_upstream =
                    get(&gs, "hasUpstream").as_bool().unwrap_or(false);
                let ahead =
                    get(&gs, "aheadCount").as_u64().unwrap_or(0);
                let running = self.git_action_running;
                let mk = {
                    let weak = weak.clone();
                    move |label: &'static str, enabled: bool, act: &'static str| {
                        let weak = weak.clone();
                        PopupMenuItem::new(label)
                            .disabled(!enabled || running)
                            .on_click(move |_, w, cx| {
                                let _ = weak.update(cx, |this, cx| match act {
                                    "pull" => {
                                        let cell = this.client.clone();
                                        let cwd = this.git_cwd();
                                        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                            if let Some(cwd) = cwd {
                                                let _ = cx.background_executor().spawn(async move {
                                                    ensure_client(&cell).and_then(|c| c.call("git.pull", json!({"cwd": cwd})))
                                                }).await;
                                                let _ = this.update(cx, |this: &mut Self, cx| {
                                                    this.fetch_git_status(cx);
                                                    this.fetch_changes(cx);
                                                    cx.notify();
                                                });
                                            }
                                        }).detach();
                                    }
                                    "push" => this.run_git_action("push", cx),
                                    "create_pr" => this.open_git_dialog(GitDialog::Pr, w, cx),
                                    "commit_push" => {
                                        this.open_git_dialog(GitDialog::Commit, w, cx);
                                        this.git_commit_action = "commit_push".to_string();
                                    }
                                    _ => this.open_git_dialog(GitDialog::Commit, w, cx),
                                });
                            })
                    }
                };
                Button::new("nav-git")
                    .ghost()
                    .compact()
                    .icon(
                        svg_icon(icons::S_BRANCH_SIMPLE)
                            .size(px(18.))
                            .text_color(glyph_idle),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.item(mk("Pull", has_repo && has_upstream, "pull"))
                            .item(PopupMenuItem::separator())
                            .item(mk("Commit…", has_repo && dirty, "commit"))
                            .item(mk("Commit & push…", has_repo && dirty, "commit_push"))
                            .item(mk("Push", has_repo && has_upstream && (ahead > 0 || dirty == false) , "push"))
                            .item(PopupMenuItem::separator())
                            .item(mk("Create PR…", has_repo && has_upstream, "create_pr"))
                    })
            })
            .child(
                div()
                    .w(px(1.))
                    .h(px(18.))
                    .mx(px(6.))
                    .bg(hairline),
            )
            .child(
                rail_btn("nav-env", icons::PANEL_TOP_OPEN, self.view_files, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_files(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-dock", icons::LAYOUT_RIGHT, self.view_changes, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_changes(cx);
                    }),
                ),
            );

        // ---- left column: #f7f7f7 well holding the rail and the floating
        // white sidebar card (bordered right+bottom, rounded corners, ~4px
        // bottom inset) exactly like upstream's app-rail-panel.
        let mut left_well = h_flex()
            .h_full()
            .bg(well_bg)
            .child(rail.border_r_1().border_color(hairline));
        if !self.sidebar_collapsed
            && !self.view_inbox
            && !self.view_tasks
            && !self.view_kanban
        {
            let side: gpui::AnyElement = if self.view_settings {
                settings_nav_el.into_any_element()
            } else if self.view_pullreq {
                pullreq_panel.into_any_element()
            } else if self.view_autom {
                autom_panel.into_any_element()
            } else if self.view_studio {
                studio_panel.into_any_element()
            } else {
                sidebar.w_full().into_any_element()
            };
            left_well = left_well.child(
                v_flex()
                    .w(px(257.))
                    .h_full()
                    .mb(px(4.))
                    .bg(sidebar_bg)
                    .border_r_1()
                    .border_b_1()
                    .border_color(hairline)
                    .rounded(px(10.))
                    .overflow_hidden()
                    .child(side),
            );
        }
        // Upstream OneTimeCoachmark: accent tooltip anchored right of the
        // rail on the wrapped button (activity = inbox, tasks). Painted last
        // so the sidebar card never covers it.
        if let Some(cm) = self.coachmark {
            let (top, title, desc) = match cm {
                "tasks" => (
                    133.0_f32,
                    "Tasks",
                    "Keep your to-dos in one place and hand them to an agent when you\u{2019}re ready.",
                ),
                _ => (
                    49.0,
                    "Activity",
                    "See running tasks, completed work, and anything that needs your attention.",
                ),
            };
            left_well = left_well.child(
                div()
                    .absolute()
                    .left(px(59.))
                    .top(px(top))
                    .w(px(240.))
                    .rounded(px(10.))
                    .bg(rgb(0xc74614))
                    .px(px(12.))
                    .py(px(10.))
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xffffff))
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(12.))
                            .text_color(hsla(0., 0., 1., 0.85))
                            .child(desc),
                    ),
            );
        }

        let mut body = h_flex()
            .flex_1()
            .h_full()
            .child(left_well)
            .child(main);
        if self.view_changes {
            body = body.child(changes_pane.border_l_1().border_color(theme.border));
        }

        // Thread context menu (right-click on a sidebar thread row): upstream
        // handleThreadContextMenu — rename, pin, handoff, fork/copy variant
        // submenus (contextMenuGroup), open-path-in-terminal, close tab,
        // archive, delete.
        let mut ctx_overlay = div().absolute().inset_0().size_full();
        if let Some((menu_tid, mx, my)) = self.ctx_menu.clone() {
            let menu_title = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "threads").iter().find_map(|t| {
                        if str_at(t, "id") == menu_tid {
                            Some(str_at(t, "title").to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default();
            let is_pinned = self.pinned_threads.contains(&menu_tid);
            let ws_path = self.thread_workspace_cwd(&menu_tid);
            let row = |id: &'static str,
                       label: String,
                       cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.ctx_menu = None;
                        this.ctx_sub = None;
                        this.ctx_confirm_delete = None;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            // Submenu parent row (upstream contextMenuGroup): one compact line
            // with a chevron that opens a flyout aligned to this row's top.
            let prow = |id: &'static str, sub: &'static str, label: &'static str, top: f32| {
                h_flex()
                    .id(id)
                    .w_full()
                    .h(px(30.))
                    .px_2()
                    .items_center()
                    .justify_between()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .child(label)
                    .child(div().text_color(theme.muted_foreground).child("›"))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        this.ctx_sub = if this.ctx_sub.as_ref().map(|s| s.0.as_str()) == Some(sub) {
                            None
                        } else {
                            Some((sub.to_string(), top))
                        };
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let sep = || div().h(px(1.)).w_full().bg(theme.border).into_any_element();
            let tid_rename = menu_tid.clone();
            let title_rename = menu_title.clone();
            let tid_pin = menu_tid.clone();
            let tid_arch = menu_tid.clone();
            let tid_close = menu_tid.clone();
            let tid_hand = menu_tid.clone();
            let tid_del = menu_tid.clone();
            let tid_del_flag = menu_tid.clone();
            // Upstream "Clear notification" row: shown when the thread has a
            // dismissible status pill. Keyed statuses store the dismissalKey;
            // Completed/Reminder store a lastVisitedAt stamp instead.
            let clear: Option<(bool, String)> = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "threads")
                        .iter()
                        .find(|t| str_at(t, "id") == menu_tid)
                        .and_then(|t| {
                            inbox_needs_you_kind(
                                t,
                                &self.dismissed_status,
                                &self.visited_threads,
                            )
                            .map(|k| {
                                if matches!(k, "approval" | "input" | "plan") {
                                    (true, dismissal_key(needs_you_label(t, k), t))
                                } else {
                                    let a = str_at(get(t, "latestTurn"), "completedAt");
                                    let b = str_at(t, "snoozeReminderAt");
                                    (
                                        false,
                                        if a >= b { a.to_string() } else { b.to_string() },
                                    )
                                }
                            })
                        })
                });
            let pin_label = if is_pinned { "Unpin thread" } else { "Pin thread" };
            let mut menu = v_flex()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(180.))
                .p_1()
                .gap_y(px(1.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                );
            if self.ctx_confirm_delete.is_some() {
                // Inline confirm like upstream confirmAndDeleteThread.
                menu = menu
                    .child(
                        div()
                            .px_2()
                            .pt_1()
                            .pb(px(2.))
                            .text_sm()
                            .child(format!("Delete thread \"{menu_title}\"?")),
                    )
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(
                                "This permanently clears conversation history for this thread.",
                            ),
                    )
                    .child(
                        h_flex()
                            .id("ctx-del-go")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Delete")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.ctx_menu = None;
                                this.ctx_sub = None;
                                this.ctx_confirm_delete = None;
                                this.delete_thread(tid_del.clone(), cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .id("ctx-del-cancel")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.ctx_confirm_delete = None;
                                cx.notify();
                            })),
                    );
            } else {
                let mut y = my + 4.0;
                menu = menu
                    .child(row("ctx-rename", "Rename thread".to_string(), Box::new(move |this, w, cx| {
                        this.open_thread(tid_rename.clone(), cx);
                        let input = cx.new(|cx| {
                            InputState::new(w, cx).placeholder("Thread title")
                        });
                        input.update(cx, |s, cx| {
                            s.set_value(title_rename.clone(), w, cx);
                        });
                        cx.subscribe_in(&input, w, |this: &mut Self, _s, event, _w, cx| {
                            if let InputEvent::PressEnter { .. } = event {
                                this.commit_rename(cx);
                            }
                        })
                        .detach();
                        let focus = input.read(cx).focus_handle(cx);
                        w.focus(&focus, cx);
                        this.rename_input = Some(input);
                        this.renaming = true;
                    })))
                    .child(row("ctx-pin", pin_label.to_string(), Box::new(move |this, _w, cx| {
                        this.toggle_pin(tid_pin.clone(), cx);
                    })))
                    .when_some(clear.clone(), |m, (is_key, val)| {
                        let tid_c = menu_tid.clone();
                        m.child(row("ctx-clearnotif", "Clear notification".to_string(), Box::new(
                            move |this, _w, cx| {
                                if is_key {
                                    this.dismissed_status.insert(tid_c.clone(), val.clone());
                                } else {
                                    this.visited_threads.insert(tid_c.clone(), val.clone());
                                }
                                this.persist_sidebar_ui(cx);
                            },
                        )))
                    })
                    .child(row("ctx-handoff", "Handoff".to_string(), Box::new(move |this, _w, cx| {
                        this.lineage(tid_hand.clone(), true, None, cx);
                    })));
                y += 93.0 + if clear.is_some() { 30.0 } else { 0.0 };
                menu = menu.child(prow("ctx-fork", "fork", "Fork", y));
                y += 34.0;
                // Copy group has separatorBefore:true upstream.
                menu = menu.child(sep()).child(prow("ctx-copy", "copy", "Copy", y));
                if let Some(path) = ws_path.clone() {
                    menu = menu.child(
                        row("ctx-open-term", "Open Path in Terminal".to_string(), Box::new(
                            move |this, w, cx| this.open_terminal_at(path.clone(), w, cx),
                        )),
                    );
                }
                menu = menu
                    .child(row("ctx-close", "Close tab".to_string(), Box::new(move |this, _w, cx| {
                        this.close_tab(tid_close.clone(), cx);
                    })))
                    .child(sep())
                    .child(row("ctx-archive", "Archive".to_string(), Box::new(move |this, _w, cx| {
                        this.open_thread(tid_arch.clone(), cx);
                        this.archive_selected(cx);
                    })))
                    .child(
                        h_flex()
                            .id("ctx-del")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Delete")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.ctx_confirm_delete = Some(tid_del_flag.clone());
                                cx.notify();
                            })),
                    );
            }
            // Backdrop as ancestor + stop-propagation shield around the popup
            // (same pattern as the palette overlay): clicks inside the menu or
            // its flyout never reach the dismiss listener.
            // The shield lives on the menu/flyout elements themselves — a
            // full-size stop-propagation wrapper would eat the backdrop clicks
            // that dismiss the menu.
            let mut inner = div().child(menu);
            // Submenu flyout: renders next to the popup, aligned to the
            // parent row that opened it.
            if self.ctx_confirm_delete.is_none() {
                if let Some((sub, sub_y)) = self.ctx_sub.clone() {
                    let mut fly = v_flex()
                        .absolute()
                        .left(px(mx + 179.))
                        .top(px(sub_y))
                        .w(px(200.))
                        .p_1()
                        .gap_y(px(1.))
                        .bg(popover_bg)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .shadow_md()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        )
                        .on_mouse_down(
                            gpui::MouseButton::Right,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        );
                    match sub.as_str() {
                        "fork" => {
                            let t_local = menu_tid.clone();
                            let t_wt = menu_tid.clone();
                            fly = fly
                                .child(row("ctx-fork-local", "Fork Into Local".to_string(), Box::new(
                                    move |this, _w, cx| {
                                        this.lineage(t_local.clone(), false, Some("local"), cx);
                                    },
                                )))
                                .child(row("ctx-fork-wt", "Fork Into New Worktree".to_string(), Box::new(
                                    move |this, _w, cx| {
                                        this.lineage(t_wt.clone(), false, Some("worktree"), cx);
                                    },
                                )));
                        }
                        "copy" => {
                            if let Some(path) = ws_path.clone() {
                                fly = fly.child(row("ctx-copy-path", "Path".to_string(), Box::new(
                                    move |_this, _w, cx| {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            path.clone(),
                                        ));
                                    },
                                )));
                            } else {
                                fly = fly.child(
                                    div()
                                        .w_full()
                                        .h(px(30.))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Path"),
                                );
                            }
                            let t_id = menu_tid.clone();
                            fly = fly.child(row("ctx-copy-id", "Thread ID".to_string(), Box::new(
                                move |_this, _w, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        t_id.clone(),
                                    ));
                                },
                            )));
                        }
                        _ => {}
                    }
                    inner = inner.child(fly);
                }
            }
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.ctx_menu = None;
                            this.ctx_sub = None;
                            this.ctx_confirm_delete = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.ctx_menu = None;
                            this.ctx_sub = None;
                            this.ctx_confirm_delete = None;
                            cx.notify();
                        }),
                    )
                    .child(inner),
            );
        }

        // ---- file-row context menu (upstream fileReferenceContextMenu):
        // Reference file in chat / Reveal in Finder / Copy path.
        if let Some((fpath, fx, fy)) = self.file_ctx_menu.clone() {
            let frow = |id: &'static str,
                        label: &'static str,
                        cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.file_ctx_menu = None;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let p_ref = fpath.clone();
            let p_reveal = fpath.clone();
            let p_copy = fpath.clone();
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.file_ctx_menu = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.file_ctx_menu = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        v_flex()
                            .absolute()
                            .left(px(fx))
                            .top(px(fy))
                            .w(px(210.))
                            .p_1()
                            .gap_y(px(1.))
                            .bg(popover_bg)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .shadow_md()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(frow("fctx-ref", "Reference file in chat", Box::new(
                                move |this, w, cx| {
                                    let name = p_ref
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(p_ref.as_str())
                                        .to_string();
                                    let draft = this.composer.read(cx).value().to_string();
                                    let next = if draft.trim().is_empty() {
                                        format!("@{name} ")
                                    } else {
                                        format!("{draft} @{name} ")
                                    };
                                    this.composer.update(cx, |s, cx| {
                                        s.set_value(next, w, cx)
                                    });
                                },
                            )))
                            .child(frow("fctx-reveal", "Reveal in Finder", Box::new(
                                move |_this, _w, _cx| {
                                    let _ = std::process::Command::new("open")
                                        .arg("-R")
                                        .arg(&p_reveal)
                                        .spawn();
                                },
                            )))
                            .child(frow("fctx-copy", "Copy path", Box::new(
                                move |_this, _w, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        p_copy.clone(),
                                    ));
                                },
                            ))),
                    ),
            );
        }

        // ---- project context menu (upstream handleProjectContextMenu):
        // Finder / kanban / copy path, edit + pin, then thread cleanup and
        // the guarded project removal row.
        if let Some((menu_pid, mx, my)) = self.proj_ctx_menu.clone() {
            let mrow = |id: &'static str,
                        label: String,
                        cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.proj_ctx_menu = None;
                        this.proj_ctx_confirm = false;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let msep = || div().h(px(1.)).w_full().bg(theme.border).into_any_element();
            let proj = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .find(|p| str_at(p, "id") == menu_pid)
                .cloned()
                .unwrap_or(Value::Null);
            let proj_title = str_at(&proj, "title").to_string();
            let proj_root = {
                let r = str_at(&proj, "workspaceRoot");
                if r.is_empty() {
                    str_at(&proj, "workspace_root").to_string()
                } else {
                    r.to_string()
                }
            };
            let proj_pinned = get(&proj, "isPinned").as_bool().unwrap_or(false);
            let has_kids = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .any(|t| {
                    str_at(t, "projectId") == menu_pid
                        && get(t, "deletedAt").is_null()
                        && get(t, "archivedAt").is_null()
                });
            let mut menu = v_flex()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(200.))
                .p_1()
                .gap_y(px(1.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                );
            if self.proj_ctx_confirm {
                // Inline confirm mirroring the thread delete branch: upstream
                // confirms before removeProject clears threads + folder.
                let pid_del = menu_pid.clone();
                menu = menu
                    .child(
                        div()
                            .px_2()
                            .pt_1()
                            .pb(px(2.))
                            .text_sm()
                            .child(format!("Remove project \"{proj_title}\"?")),
                    )
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(if has_kids {
                                "Threads in this folder are deleted first."
                            } else {
                                "This removes the project from the sidebar."
                            }),
                    )
                    .child(
                        h_flex()
                            .id("pctx-del-go")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Remove")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.proj_ctx_menu = None;
                                this.proj_ctx_confirm = false;
                                this.delete_project(pid_del.clone(), cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .id("pctx-del-cancel")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.proj_ctx_confirm = false;
                                cx.notify();
                            })),
                    );
            } else {
                let p_reveal = proj_root.clone();
                let p_copy = proj_root.clone();
                let pid_pin = menu_pid.clone();
                let pid_edit = menu_pid.clone();
                let title_edit = proj_title.clone();
                let pid_arch = menu_pid.clone();
                let pid_del_t = menu_pid.clone();
                menu = menu
                    .child(mrow("pctx-finder", "Open in Finder".into(), Box::new(
                        move |_this, _w, _cx| {
                            if !p_reveal.is_empty() {
                                let _ = std::process::Command::new("open")
                                    .arg("-R")
                                    .arg(&p_reveal)
                                    .spawn();
                            }
                        },
                    )))
                    .child(mrow("pctx-kanban", "Open in kanban".into(), Box::new(
                        move |this, _w, cx| {
                            this.view_files = false;
                            this.view_dev = false;
                            this.view_inbox = false;
                            this.view_tasks = false;
                            this.view_pullreq = false;
                            this.view_autom = false;
                            this.view_studio = false;
                            this.view_spaces = false;
                            this.view_kanban = true;
                            cx.notify();
                        },
                    )))
                    .child(mrow("pctx-copy", "Copy path".into(), Box::new(
                        move |_this, _w, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                p_copy.clone(),
                            ));
                        },
                    )))
                    .child(msep())
                    .child(mrow("pctx-edit", "Edit project".into(), Box::new(
                        move |this, w, cx| {
                            this.start_project_rename(pid_edit.clone(), title_edit.clone(), w, cx);
                        },
                    )))
                    .child(mrow("pctx-pin", if proj_pinned {
                        "Unpin project".to_string()
                    } else {
                        "Pin project".to_string()
                    }, Box::new(move |this, _w, cx| {
                        this.toggle_project_pin(pid_pin.clone(), proj_pinned, cx);
                    })));
                if has_kids {
                    menu = menu
                        .child(msep())
                        .child(mrow("pctx-arch", "Archive all threads".into(), Box::new(
                            move |this, _w, cx| {
                                this.project_threads_cmd(pid_arch.clone(), "thread.archive", cx);
                            },
                        )))
                        .child(mrow("pctx-delt", "Delete all threads".into(), Box::new(
                            move |this, _w, cx| {
                                this.project_threads_cmd(pid_del_t.clone(), "thread.delete", cx);
                            },
                        )));
                }
                menu = menu.child(msep()).child(
                    h_flex()
                        .id("pctx-remove")
                        .w_full()
                        .h(px(30.))
                        .px_2()
                        .items_center()
                        .rounded_md()
                        .text_sm()
                        .text_color(theme.danger)
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child("Remove")
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_confirm = true;
                            cx.notify();
                        })),
                );
            }
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_menu = None;
                            this.proj_ctx_confirm = false;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_menu = None;
                            this.proj_ctx_confirm = false;
                            cx.notify();
                        }),
                    )
                    .child(menu),
            );
        }

        // ---- thread hover card (upstream ThreadHoverCardContent): title +
        // relative time header, then status / project / cwd / model rows.
        // Zero-size anchor: absolute children position from window origin
        // without the container itself intercepting pointer events.
        let mut hc_overlay = div().absolute().left_0().top_0().w_0().h_0();
        if let Some((hc_tid, hx, hy)) = self.hover_card.clone() {
            let threads_arr = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads");
            if let Some(t) = threads_arr.iter().find(|t| str_at(t, "id") == hc_tid).cloned() {
                let title = str_at(&t, "title");
                let title = if title.is_empty() { "(untitled)".to_string() } else { title.to_string() };
                let rel = rel_time(str_at(&t, "updatedAt"));
                let status = inbox_needs_you_kind(&t, &self.dismissed_status, &self.visited_threads)
                    .map(|k| match k {
                        "approval" => "Needs approval",
                        "input" => "Awaiting input",
                        "plan" => "Plan ready",
                        "failed" => "Failed",
                        _ => "Unread",
                    })
                    .or_else(|| if thread_actively_working(&t) { Some("Working") } else { None })
                    .unwrap_or("Idle");
                let pid = str_at(&t, "projectId");
                let pname = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == pid)
                    .map(|p| str_at(p, "name").to_string())
                    .unwrap_or_default();
                let cwd = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == pid)
                    .map(|p| {
                        let r = str_at(p, "workspaceRoot");
                        if r.is_empty() { str_at(p, "root").to_string() } else { r.to_string() }
                    })
                    .unwrap_or_default();
                let model = {
                    let s = get(&t, "session");
                    let m = str_at(s, "modelName");
                    let m2 = str_at(s, "model");
                    let p = str_at(s, "providerName");
                    let pick = if !m.is_empty() { m } else { m2 };
                    if pick.is_empty() && p.is_empty() { String::new() } else { format!("{p} {pick}").trim().to_string() }
                };
                let mut card = v_flex()
                    .w(px(300.))
                    .p_2()
                    .gap_1()
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .child(
                        h_flex()
                            .items_baseline()
                            .justify_between()
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(13.)).child(title))
                            .child(div().text_size(px(11.)).text_color(theme.muted_foreground).child(rel)),
                    )
                    .child(div().text_size(px(12.)).text_color(theme.accent).child(status));
                if !pname.is_empty() {
                    card = card.child(div().text_size(px(12.)).text_color(theme.foreground).child(pname));
                }
                if !cwd.is_empty() {
                    card = card.child(div().text_size(px(11.)).text_color(theme.muted_foreground).whitespace_nowrap().overflow_hidden().child(cwd));
                }
                if !model.is_empty() {
                    card = card.child(div().text_size(px(11.)).text_color(theme.muted_foreground).child(model));
                }
                let vs = window.viewport_size();
                // Mounted separately (small box only) — a full-size overlay
                // would swallow the pointer events the card depends on.
                hc_overlay = hc_overlay.child(
                    div()
                        .absolute()
                        .left(px((hx + 14.).min(f32::from(vs.width) - 320.)))
                        .top(px(hy.min(f32::from(vs.height) - 220.)))
                        .child(card),
                );
            }
        }

        // ---- workspace search palette (⌘K / ⌘P / ⇧⌘F): upstream
        // SidebarSearchPalette + WorkspaceSearchPalette — bare h-11 input,
        // max-w-lg popup anchored near the top, 28px result rows grouped by
        // section, arrows+enter keyboard navigation, backdrop/Esc dismiss.
        // ---- project hover card (upstream ProjectHoverCardContent): icon +
        // name + pin toggle header, chat count, path, "Edit project" row.
        let mut pc_overlay = div().absolute().left_0().top_0().w_0().h_0();
        if let Some((pc_pid, pcx, pcy)) = self.project_card.clone() {
            let projects_arr = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects");
            if let Some(p) = projects_arr
                .iter()
                .find(|p| str_at(p, "id") == pc_pid)
                .cloned()
            {
                let name = {
                    let t = str_at(&p, "title");
                    if t.is_empty() { "Untitled project".to_string() } else { t.to_string() }
                };
                let pinned = get(&p, "isPinned").as_bool().unwrap_or(false);
                let root = {
                    let r = str_at(&p, "workspaceRoot");
                    if r.is_empty() {
                        str_at(&p, "workspace_root").to_string()
                    } else {
                        r.to_string()
                    }
                };
                let n_chats = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .filter(|t| {
                        str_at(t, "projectId") == pc_pid
                            && get(t, "deletedAt").is_null()
                            && get(t, "archivedAt").is_null()
                    })
                    .count();
                let pid_pin = pc_pid.clone();
                let pid_edit = pc_pid.clone();
                let name_edit = name.clone();
                let pin_icon = if pinned { icons::PIN_FILL } else { icons::PIN };
                let divider = |d: Div| d;
                let _ = divider;
                let card = v_flex()
                    .w(px(300.))
                    .p_1()
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .child(
                        h_flex()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_size(px(13.))
                                    .flex_1()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .id("pc-pin")
                                    .cursor_pointer()
                                    .rounded_sm()
                                    .p_1()
                                    .text_color(if pinned {
                                        theme.foreground
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .hover(|d| d.text_color(theme.foreground))
                                    .child(svg_icon(pin_icon).size(px(12.)))
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.project_card = None;
                                        this.toggle_project_pin(pid_pin.clone(), pinned, cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .h(px(28.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{n_chats} {}",
                                if n_chats == 1 { "chat" } else { "chats" }
                            )),
                    )
                    .child(div().h(px(1.)).w_full().bg(theme.border));
                let mut card = card.child(
                    h_flex()
                        .h(px(28.))
                        .px_2()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(if root.is_empty() {
                                    "No workspace folder".to_string()
                                } else {
                                    root
                                }),
                        ),
                );
                card = card
                    .child(div().h(px(1.)).w_full().bg(theme.border))
                    .child(
                        h_flex()
                            .id("pc-edit")
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .text_size(px(12.))
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Edit project")
                            .on_click(cx.listener(move |this, _ev, w, cx| {
                                this.project_card = None;
                                this.start_project_rename(
                                    pid_edit.clone(),
                                    name_edit.clone(),
                                    w,
                                    cx,
                                );
                            })),
                    );
                let vs = window.viewport_size();
                pc_overlay = pc_overlay.child(
                    div()
                        .absolute()
                        .left(px((pcx + 14.).min(f32::from(vs.width) - 320.)))
                        .top(px(pcy.min(f32::from(vs.height) - 180.)))
                        .child(card),
                );
            }
        }

        // ---- shared RenameDialog (upstream RenameDialog): centered card with
        // a single field and Cancel/Save — covers the thread rename (mounted
        // for the first time here) and the project "Edit project" action.
        // Diff-line blame popover (upstream DiffLineBlamePopover):
        // summary + author + date card at the clicked diff line.
        let mut bp_overlay = div().absolute().inset_0().size_full();
        if let Some((bv, bx, by)) = self.blame_pop.clone() {
            bp_overlay = bp_overlay.child(
                div()
                    .size_full()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(
                        |this, _ev, _w, cx| {
                            this.blame_pop = None;
                            cx.notify();
                        },
                    )),
            );
            let uncommitted = get(&bv, "uncommitted").as_bool().unwrap_or(false);
            let summary = {
                let t = str_at(&bv, "summary");
                if t.is_empty() && uncommitted { "Uncommitted changes".to_string() } else { t.to_string() }
            };
            let author = str_at(&bv, "author").to_string();
            let when = str_at(&bv, "authorTime").to_string();
            let sha = str_at(&bv, "shortSha").to_string();
            let vs = window.viewport_size();
            let card = v_flex()
                .w(px(264.))
                .gap_1()
                .p_2()
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if uncommitted { "Uncommitted changes".to_string() } else { summary }),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .flex_1()
                                .child(if uncommitted { "Local edit".to_string() } else { author }),
                        )
                        .child(if uncommitted { "".to_string() } else { when }),
                )
                .when(!sha.is_empty() && !uncommitted, |d| {
                    d.child(
                        div()
                            .text_size(px(11.))
                            .font_family(theme.mono_font_family.clone())
                            .text_color(theme.muted_foreground)
                            .child(sha),
                    )
                });
            bp_overlay = bp_overlay.child(
                div()
                    .absolute()
                    .left(px((bx + 8.).min(f32::from(vs.width) - 280.)))
                    .top(px((by + 8.).min(f32::from(vs.height) - 120.)))
                    .child(card),
            );
        }

        let mut ren_overlay = div().absolute().inset_0().size_full();
        if self.renaming || self.proj_rename.is_some() {
            let is_project = self.proj_rename.is_some();
            let dlg_title = if is_project { "Edit project" } else { "Rename chat" };
            let input = if is_project {
                self.proj_rename_input.clone()
            } else {
                self.rename_input.clone()
            };
            let can_save = input
                .as_ref()
                .map(|i| !i.read(cx).value().trim().is_empty())
                .unwrap_or(false);
            let mut dlg = v_flex()
                .w(px(448.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.renaming = false;
                        this.rename_input = None;
                        this.proj_rename = None;
                        this.proj_rename_input = None;
                        cx.stop_propagation();
                        cx.notify();
                    } else if ev.keystroke.key == "enter" {
                        if this.proj_rename.is_some() {
                            this.commit_project_rename(cx);
                        } else {
                            this.commit_rename(cx);
                        }
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .px(px(20.))
                        .pt(px(20.))
                        .pb(px(8.))
                        .text_size(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(dlg_title),
                );
            if let Some(inp) = input {
                dlg = dlg.child(
                    div()
                        .mx(px(20.))
                        .my(px(8.))
                        .child(Input::new(&inp)),
                );
            }
            dlg = dlg.child(
                h_flex()
                    .w_full()
                    .px(px(20.))
                    .py(px(16.))
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("ren-cancel")
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.renaming = false;
                                this.rename_input = None;
                                this.proj_rename = None;
                                this.proj_rename_input = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ren-save")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if !can_save {
                                    return;
                                }
                                if this.proj_rename.is_some() {
                                    this.commit_project_rename(cx);
                                } else {
                                    this.commit_rename(cx);
                                }
                            })),
                    ),
            );
            ren_overlay = ren_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black().opacity(0.3))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.renaming = false;
                            this.rename_input = None;
                            this.proj_rename = None;
                            this.proj_rename_input = None;
                            cx.notify();
                        }),
                    )
                    .child(dlg),
            );
        }

        let mut palette_overlay = div().absolute().inset_0().size_full();
        if let Some(mode) = self.palette_mode {
            let query = self
                .palette_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            let items = self.palette_items(&query);
            let sel = self.palette_sel;
            let mut popup = v_flex()
                .w(px(512.))
                .mt(px(120.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                    match ev.keystroke.key.as_str() {
                        "escape" => this.close_palette(window, cx),
                        "down" => {
                            let q = this
                                .palette_input
                                .as_ref()
                                .map(|i| i.read(cx).value().to_string())
                                .unwrap_or_default();
                            let len = this.palette_items(&q).len();
                            this.palette_sel = (this.palette_sel + 1).min(len.saturating_sub(1));
                            cx.notify();
                        }
                        "up" => {
                            this.palette_sel = this.palette_sel.saturating_sub(1);
                            cx.notify();
                        }
                        "backspace" => {
                            if let Some(input) = this.palette_input.clone() {
                                let mut q = input.read(cx).value().to_string();
                                if q.pop().is_some() {
                                    input.update(cx, |s, cx| {
                                        s.set_value(q.clone(), window, cx)
                                    });
                                    this.palette_query_changed(cx);
                                }
                            }
                        }
                        _ => {
                            // Printable characters: insert manually — the
                            // platform input handler is not reliably bound for
                            // late-mounted palette inputs, so key_char chars
                            // would bubble up unconsumed.
                            if let Some(ch) = ev.keystroke.key_char.as_ref() {
                                let m = ev.keystroke.modifiers;
                                if !m.control && !m.platform && !m.function {
                                    if let Some(input) = this.palette_input.clone() {
                                        let mut q =
                                            input.read(cx).value().to_string();
                                        q.push_str(ch);
                                        input.update(cx, |s, cx| {
                                            s.set_value(q, window, cx)
                                        });
                                        this.palette_query_changed(cx);
                                    }
                                }
                            }
                        }
                    }
                    cx.stop_propagation();
                }));
            if let Some(input) = self.palette_input.clone() {
                popup = popup.child(
                    div()
                        .h(px(44.))
                        .px(px(14.))
                        .border_b_1()
                        .border_color(theme.border)
                        .child(Input::new(&input).appearance(false)),
                );
            }
            let mut list = v_flex()
                .id("palette-list")
                .max_h(px(448.))
                .overflow_y_scroll()
                .p_1()
                .gap_y(px(1.));
            let mut last_group = "";
            let group_label = |kind: &str, query_empty: bool| -> &'static str {
                match kind {
                    "project" => "Projects",
                    "thread" => {
                        if query_empty {
                            "Recent chats"
                        } else {
                            "Threads"
                        }
                    }
                    "snippet" => "Matches",
                    _ => "Files",
                }
            };
            for (i, (kind, label, sub, _payload)) in items.iter().enumerate() {
                let group = group_label(kind, query.trim().is_empty());
                if group != last_group {
                    last_group = group;
                    list = list.child(
                        div()
                            .px(px(10.))
                            .pt(px(6.))
                            .pb(px(4.))
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(group),
                    );
                }
                let icon_data: &[u8] = match kind.as_str() {
                    "directory" | "project" => icons::FOLDER,
                    "thread" => icons::S_CHAT_BUBBLES,
                    "snippet" => icons::SEARCH,
                    _ => icons::FILE_TEXT,
                };
                let mut r = h_flex()
                    .id(ElementId::Name(format!("pal-{i}").into()))
                    .w_full()
                    .h(px(28.))
                    .px(px(10.))
                    .items_center()
                    .gap_2()
                    .rounded_lg()
                    .cursor_pointer()
                    .child(svg_icon(icon_data).size(px(14.)).text_color(theme.muted_foreground))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_size(px(13.))
                            .child(label.clone()),
                    );
                if !sub.is_empty() {
                    r = r.child(
                        div()
                            .max_w(px(220.))
                            .overflow_hidden()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(sub.clone()),
                    );
                }
                if i == sel {
                    r = r.bg(theme.secondary);
                }
                r = r.on_click(cx.listener(move |this, _ev, window, cx| {
                    this.palette_sel = i;
                    this.palette_activate(window, cx);
                }));
                list = list.child(r);
            }
            if items.is_empty() {
                let msg = match mode {
                    "files" => {
                        if query.trim().is_empty() {
                            "Type to search for files"
                        } else {
                            "No matching files"
                        }
                    }
                    "snippets" => {
                        if query.trim().len() < 2 {
                            "Type at least 2 characters to search code"
                        } else {
                            "No matches"
                        }
                    }
                    _ => "No results.",
                };
                list = list.child(
                    div()
                        .px(px(10.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(msg),
                );
            }
            popup = popup.child(list);
            palette_overlay = palette_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, window, cx| this.close_palette(window, cx)),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(popup),
                    ),
            );
        }

        // Shortcut recorder dialog (Settings > Keyboard shortcuts).
        let mut sc_overlay = div().absolute().inset_0().size_full();
        if let Some(rec) = self.sc_rec.as_ref().map(|r| ScRec {
            label: r.label.clone(),
            commands: r.commands.clone(),
            numbered: r.numbered,
            replacing: r.replacing.clone(),
            captured: r.captured.clone(),
        }) {
            let (problem, conflicts) = self.sc_evaluate(&rec);
            let captured_label = rec.captured.as_ref().map(sc_fmt_captured);
            let ready = rec.captured.is_some() && problem.is_none();
            let has_conflicts = !conflicts.is_empty();
            let can_save = ready;
            let status_text = if let Some(p) = &problem {
                p.clone()
            } else if ready {
                if has_conflicts {
                    let names: Vec<String> = conflicts
                        .iter()
                        .map(|(c, _)| Self::sc_command_label(c))
                        .collect();
                    format!(
                        "{} already runs \"{}\". Saving moves it here.",
                        captured_label.clone().unwrap_or_default(),
                        names.join("\" and \""),
                    )
                } else {
                    format!(
                        "{} is available.",
                        captured_label.clone().unwrap_or_default()
                    )
                }
            } else if rec.numbered {
                "Hold the modifiers then press any number key.".to_string()
            } else {
                "Use \u{2318}, \u{2325} or \u{2303} with any key, or an F-key on its own."
                    .to_string()
            };
            let status_color: Hsla = if problem.is_some() {
                gpui::hsla(0.0, 0.72, 0.51, 1.0)
            } else if has_conflicts {
                gpui::hsla(0.09, 0.85, 0.47, 1.0)
            } else {
                theme.muted_foreground
            };
            let desc = if rec.replacing.is_empty() {
                "Choose the keys that run it."
            } else {
                "Press new keys to change its shortcut."
            };
            sc_overlay = sc_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.sc_rec = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(
                                v_flex()
                                    .w(px(400.))
                                    .gap_3()
                                    .px_4()
                                    .pt_4()
                                    .pb_3()
                                    .rounded_xl()
                                    .bg(popover_bg)
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .w(px(36.))
                                                    .h(px(36.))
                                                    .flex_shrink_0()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(9.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .bg(theme.muted)
                                                    .child(
                                                        svg_icon(icons::SEARCH)
                                                            .size(px(16.))
                                                            .text_color(theme.foreground),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .min_w_0()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(15.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .child(rec.label.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(desc),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .min_h(px(88.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_xl()
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(theme.muted)
                                            .px_3()
                                            .child(if let Some(l) = &captured_label {
                                                sc_chip(l.clone(), &theme).into_any_element()
                                            } else {
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(if rec.numbered {
                                                        "1\u{2013}9"
                                                    } else {
                                                        "Type a shortcut"
                                                    })
                                                    .into_any_element()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .min_h(px(32.))
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(status_color)
                                            .child(status_text),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(div().flex_1())
                                            .child(
                                                h_flex()
                                                    .id("sc-cancel")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        |this, _ev, _w, cx| {
                                                            this.sc_rec = None;
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .h(px(32.))
                                                            .items_center()
                                                            .rounded(px(8.))
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .text_size(px(13.))
                                                            .child("Cancel"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("sc-save")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        |this, _ev, _w, cx| {
                                                            this.sc_save(cx);
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .h(px(32.))
                                                            .items_center()
                                                            .rounded(px(8.))
                                                            .bg(if can_save {
                                                                theme.foreground
                                                            } else {
                                                                theme.muted
                                                            })
                                                            .text_size(px(13.))
                                                            .text_color(theme.background)
                                                            .child("Save"),
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
            );
        }

        // Feedback dialog (upstream FeedbackDialog): centered card over a
        // dimmed backdrop — category pills, details textarea, diagnostics
        // note, full-width Submit.
        let mut fb_overlay = div().absolute().inset_0().size_full();
        if self.feedback_open {
            let can_submit = self
                .feedback_input
                .as_ref()
                .map(|i| !i.read(cx).value().trim().is_empty())
                .unwrap_or(false)
                && !self.feedback_sending;
            let mut pills = h_flex().w_full().px(px(20.)).gap_1p5().flex_wrap();
            for (i, (_v, label, _lead)) in Self::FEEDBACK_CATEGORIES.iter().enumerate()
            {
                let sel = self.feedback_category == Some(i);
                pills = pills.child(
                    div()
                        .id(ElementId::Name(format!("fb-cat-{i}").into()))
                        .cursor_pointer()
                        .rounded(px(999.))
                        .px(px(14.))
                        .py(px(5.))
                        .text_size(px(13.))
                        .border_1()
                        .border_color(theme.border)
                        .text_color(if sel {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .bg(if sel {
                            theme.secondary
                        } else {
                            gpui::transparent_black()
                        })
                        .child(format!("{} {}", if sel { "\u{2212}" } else { "+" }, label))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            this.feedback_category =
                                if this.feedback_category == Some(i) {
                                    None
                                } else {
                                    Some(i)
                                };
                            cx.notify();
                        })),
                );
            }
            let mut card = v_flex()
                .w(px(576.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" && !this.feedback_sending {
                        this.feedback_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .px(px(20.))
                        .pt(px(20.))
                        .pb(px(12.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex_1()
                                .child("Share feedback"),
                        )
                        .child(
                            div()
                                .id("fb-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    if !this.feedback_sending {
                                        this.feedback_open = false;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(pills);
            if let Some(input) = self.feedback_input.clone() {
                card = card.child(
                    div()
                        .mx(px(20.))
                        .mt(px(12.))
                        .min_h(px(128.))
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(8.))
                        .px_2()
                        .py_1()
                        .child(Input::new(&input).appearance(false)),
                );
            }
            card = card
                .child(
                    div()
                        .px(px(20.))
                        .pt(px(12.))
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(
                            "Diagnostics include app version, OS, provider/model, \
                             modes, and session state — never prompts, messages, \
                             paths, or logs.",
                        ),
                )
                .child(
                    div()
                        .mx(px(20.))
                        .my(px(16.))
                        .h(px(36.))
                        .id("fb-submit")
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .bg(if can_submit {
                            theme.foreground
                        } else {
                            theme.muted
                        })
                        .text_color(theme.background)
                        .text_size(px(13.))
                        .cursor_pointer()
                        .child(if self.feedback_sending {
                            "Sending…"
                        } else {
                            "Submit"
                        })
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.submit_feedback(cx)
                        })),
                );
            fb_overlay = fb_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            if !this.feedback_sending {
                                this.feedback_open = false;
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| {
                                    cx.stop_propagation()
                                }),
                            )
                            .child(card),
                    ),
            );
        }

        // Keybindings reference dialog (upstream ShortcutsDialog): search
        // field + filtered title/shortcut rows over a dimmed backdrop.
        let mut scd_overlay = div().absolute().inset_0().size_full();
        if self.sc_dialog_open {
            let query = self
                .sc_dialog_input
                .as_ref()
                .map(|i| i.read(cx).value().to_lowercase())
                .unwrap_or_default();
            let mut list = v_flex()
                .id("scd-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(12.))
                .pb(px(12.));
            let mut shown = 0;
            for (title, _desc, kbd, commands) in SHORTCUT_ROWS.iter() {
                if !query.trim().is_empty()
                    && !title.to_lowercase().contains(query.trim())
                {
                    continue;
                }
                shown += 1;
                let label = self.sc_row_label(commands, kbd);
                list = list.child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .rounded_md()
                        .px_3()
                        .py_1p5()
                        .child(div().text_size(px(13.)).child(*title))
                        .child(if label.is_empty() {
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child("—")
                                .into_any_element()
                        } else {
                            sc_chip(label, &theme).into_any_element()
                        }),
                );
            }
            if shown == 0 {
                list = list.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(format!("No shortcuts match \u{201c}{query}\u{201d}")),
                );
            }
            let mut card = v_flex()
                .w(px(560.))
                .h(px(520.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.sc_dialog_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .px(px(16.))
                        .pt(px(16.))
                        .pb(px(8.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex_1()
                                .child("Keybindings"),
                        )
                        .child(
                            div()
                                .id("scd-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.sc_dialog_open = false;
                                    cx.notify();
                                })),
                        ),
                );
            if let Some(input) = self.sc_dialog_input.clone() {
                card = card.child(
                    div()
                        .mx(px(16.))
                        .mb(px(8.))
                        .h(px(36.))
                        .px_3()
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(8.))
                        .child(Input::new(&input).appearance(false)),
                );
            }
            card = card.child(list);
            scd_overlay = scd_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.sc_dialog_open = false;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| {
                                    cx.stop_propagation()
                                }),
                            )
                            .child(card),
                    ),
            );
        }

        // App tour (upstream AppTour): dimmed workspace, accent ring on the
        // anchored control, step card beside it, dots + Back/Next, ✕/Escape.
        let mut tour_overlay = div().absolute().inset_0().size_full();
        if let Some(step_idx) = self.tour_step.filter(|i| *i < TOUR_STEPS.len()) {
            let (place, title, desc, anchor) = TOUR_STEPS[step_idx];
            let last = step_idx + 1 == TOUR_STEPS.len();
            let mut dots = h_flex().gap(px(6.)).items_center();
            for i in 0..TOUR_STEPS.len() {
                dots = dots.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(if i == step_idx {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        }),
                );
            }
            let mut card = v_flex()
                .w(px(300.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    match ev.keystroke.key.as_str() {
                        "escape" => {
                            this.tour_step = None;
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "arrowright" | "enter" => {
                            this.tour_step = Some(
                                (this.tour_step.unwrap_or(0) + 1).min(TOUR_STEPS.len() - 1),
                            );
                            cx.notify();
                        }
                        "arrowleft" => {
                            this.tour_step = Some(this.tour_step.unwrap_or(1).saturating_sub(1));
                            cx.notify();
                        }
                        _ => {}
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(place),
                        )
                        .child(
                            div()
                                .id("tour-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.tour_step = None;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(desc),
                )
                .child(
                    h_flex()
                        .w_full()
                        .pt_2()
                        .items_center()
                        .child(dots)
                        .child(div().flex_1())
                        .when(step_idx > 0, |d| {
                            d.child(
                                div()
                                    .id("tour-back")
                                    .cursor_pointer()
                                    .h(px(28.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .mr_1()
                                    .text_size(px(13.))
                                    .hover(|h| h.bg(theme.secondary))
                                    .child("Back")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.tour_step =
                                            Some(this.tour_step.unwrap_or(1).saturating_sub(1));
                                        cx.notify();
                                    })),
                            )
                        })
                        .child(
                            div()
                                .id("tour-next")
                                .cursor_pointer()
                                .h(px(28.))
                                .px_3()
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .bg(theme.foreground)
                                .text_color(theme.background)
                                .text_size(px(13.))
                                .child(if last { "Done" } else { "Next" })
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    this.tour_step = if last {
                                        None
                                    } else {
                                        Some(step_idx + 1)
                                    };
                                    cx.notify();
                                })),
                        ),
                );
            let mut stage = v_flex()
                .size_full()
                .bg(rgba(0x00000066))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, _w, cx| {
                        this.tour_step = None;
                        cx.notify();
                    }),
                );
            if let Some((x0, y0, x1, y1)) = anchor {
                stage = stage
                    .child(
                        div()
                            .absolute()
                            .left(px(x0 - 4.))
                            .top(px(y0 - 4.))
                            .w(px(x1 - x0 + 8.))
                            .h(px(y1 - y0 + 8.))
                            .rounded(px(10.))
                            .border_2()
                            .border_color(theme.primary),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(x1 + 14.))
                            .top(px(y0 - 4.))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(card),
                    );
            } else {
                stage = stage.items_center().justify_center().child(
                    div()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        )
                        .child(card),
                );
            }
            tour_overlay = tour_overlay.child(stage);
        }

        // ---- git action dialog (upstream GitCommitDialog/GitCreatePrDialog):
        // centered card over a dim backdrop; Escape/backdrop close; the
        // commit variant also picks which stacked action to run.
        let mut git_overlay = div().absolute().inset_0().size_full();
        if let Some(gkind) = self.git_dialog {
            let mut card = v_flex()
                .w(px(420.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.close_git_dialog(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(match gkind {
                                    GitDialog::Commit => "Commit changes",
                                    GitDialog::Pr => "Create pull request",
                                }),
                        )
                        .child(
                            div()
                                .id("gitdlg-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.close_git_dialog(cx);
                                })),
                        ),
                );
            let branch = get(&self.git_status.clone().unwrap_or(Value::Null), "branch")
                .as_str()
                .unwrap_or("")
                .to_string();
            if !branch.is_empty() {
                card = card.child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(
                            svg_icon(icons::S_BRANCH_SIMPLE)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(branch),
                        ),
                );
            }
            if gkind == GitDialog::Commit {
                if let Some(input) = self.git_msg_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(36.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                let actions: [(&str, &str, &str); 4] = [
                    ("commit", "Commit", "Commit staged changes on this branch."),
                    ("commit_push", "Commit & push", "Commit, then push to the upstream branch."),
                    ("commit_new_branch", "Commit on new branch", "Create a feature branch, then commit there."),
                    ("create_pr", "Create PR", "Commit, push, and open a pull request."),
                ];
                for (id, label, desc) in actions {
                    let sel = self.git_commit_action == id;
                    let act = id.to_string();
                    card = card.child(
                        h_flex()
                            .id(ElementId::Name(format!("gitact-{id}").into()))
                            .w_full()
                            .items_center()
                            .gap_2()
                            .rounded(px(8.))
                            .px_2()
                            .py_1p5()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.git_commit_action = act.clone();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(14.))
                                    .h(px(14.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if sel { theme.primary } else { theme.border })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(sel, |d| {
                                        d.child(
                                            div().w(px(8.)).h(px(8.)).rounded_full().bg(theme.primary),
                                        )
                                    }),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div().text_size(px(13.)).child(label),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child(desc),
                                    ),
                            ),
                    );
                }
            } else {
                if let Some(input) = self.git_pr_title_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(36.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                if let Some(input) = self.git_pr_body_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(80.))
                            .px_3()
                            .py_2()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                let draft = self.git_pr_draft;
                card = card.child(
                    h_flex()
                        .id("gitdlg-draft")
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.git_pr_draft = !this.git_pr_draft;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(14.))
                                .h(px(14.))
                                .rounded(px(4.))
                                .border_1()
                                .border_color(if draft { theme.primary } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(draft, |d| {
                                    d.child(
                                        svg_icon(icons::CHECK)
                                            .size(px(10.))
                                            .text_color(theme.primary),
                                    )
                                }),
                        )
                        .child(div().text_size(px(13.)).child("Create as draft")),
                );
            }
            let run_action = if gkind == GitDialog::Commit {
                self.git_commit_action.clone()
            } else {
                "create_pr".to_string()
            };
            let run_label: &str = match run_action.as_str() {
                "commit_push" => "Commit & push",
                "commit_new_branch" => "Commit on new branch",
                "create_pr" => "Create PR",
                _ => "Commit",
            };
            card = card.child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .pt_1()
                    .child(
                        div()
                            .id("gitdlg-cancel")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.close_git_dialog(cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("gitdlg-run")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .text_color(theme.primary_foreground)
                            .bg(theme.primary)
                            .when(self.git_action_running, |d| d.opacity(0.6))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                let a = run_action.clone();
                                this.run_git_action(&a, cx);
                            }))
                            .child(if self.git_action_running {
                                "Running…"
                            } else {
                                run_label
                            }),
                    ),
            );
            git_overlay = git_overlay
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _e, _w, cx| {
                                this.close_git_dialog(cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    |_e, _w, cx| cx.stop_propagation(),
                                )
                                .child(card),
                        ),
                );
        }

        // Automation create/edit dialog (upstream automation form).
        let mut autom_overlay = div().absolute().inset_0().size_full();
        if let Some(form) = self.autom_dialog.as_ref() {
            let editing = form.edit_id.is_some();
            let name_in = form.name.clone();
            let prompt_in = form.prompt.clone();
            let time_in = form.time.clone();
            let extra_in = form.extra.clone();
            let kind = form.kind;
            let mut card = v_flex()
                .w(px(440.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.close_autom_dialog(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if editing {
                            "Edit automation"
                        } else {
                            "New automation"
                        }),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(34.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&name_in).appearance(false)),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(56.))
                        .px_3()
                        .py_2()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&prompt_in).appearance(false)),
                );
            // Project picker (radio list, upstream AutomationHeadingFields).
            let projects = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects");
            card = card.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("Project"),
            );
            for (i, p) in projects.iter().enumerate().take(6) {
                let sel = form.proj == i;
                card = card.child(
                    h_flex()
                        .id(ElementId::Name(format!("autproj-{i}").into()))
                        .items_center()
                        .gap_2()
                        .rounded(px(6.))
                        .px_2()
                        .py_1()
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            if let Some(f) = this.autom_dialog.as_mut() {
                                f.proj = i;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(12.))
                                .h(px(12.))
                                .rounded_full()
                                .border_1()
                                .border_color(if sel { theme.primary } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(sel, |d| {
                                    d.child(
                                        div()
                                            .w(px(6.))
                                            .h(px(6.))
                                            .rounded_full()
                                            .bg(theme.primary),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .overflow_hidden()
                                .child(str_at(p, "title").to_string()),
                        ),
                );
            }
            // Schedule kind chips.
            card = card.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("Schedule"),
            );
            let mut chip_row = h_flex().gap_1().flex_wrap();
            for (i, label) in AUTOM_KINDS.iter().enumerate() {
                let sel = kind == i;
                chip_row = chip_row.child(
                    div()
                        .id(ElementId::Name(format!("autkind-{i}").into()))
                        .h(px(24.))
                        .px_2()
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(if sel { theme.primary } else { theme.border })
                        .cursor_pointer()
                        .when(sel, |d| d.bg(theme.primary.opacity(0.12)))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            if let Some(f) = this.autom_dialog.as_mut() {
                                f.kind = i;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(if sel { theme.primary } else { theme.foreground })
                                .child(*label),
                        ),
                );
            }
            card = card.child(chip_row);
            // Per-kind fields.
            if matches!(kind, 2 | 3 | 4) {
                card = card.child(
                    div()
                        .w(px(140.))
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&time_in).appearance(false)),
                );
            }
            if kind == 4 {
                let mut day_row = h_flex().gap_1();
                for (i, d) in AUTOM_DAYS.iter().enumerate() {
                    let sel = form.day == i;
                    day_row = day_row.child(
                        div()
                            .id(ElementId::Name(format!("autday-{i}").into()))
                            .w(px(34.))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if sel { theme.primary } else { theme.border })
                            .cursor_pointer()
                            .when(sel, |d| d.bg(theme.primary.opacity(0.12)))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                if let Some(f) = this.autom_dialog.as_mut() {
                                    f.day = i;
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if sel {
                                        theme.primary
                                    } else {
                                        theme.foreground
                                    })
                                    .child(*d),
                            ),
                    );
                }
                card = card.child(day_row);
            }
            if matches!(kind, 0 | 1 | 5) {
                card = card.child(
                    div()
                        .w_full()
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&extra_in).appearance(false)),
                );
            }
            // Enabled toggle row.
            let en = form.enabled;
            card = card.child(
                h_flex()
                    .id("autmenabled")
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        if let Some(f) = this.autom_dialog.as_mut() {
                            f.enabled = !f.enabled;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(if en { theme.primary } else { theme.border })
                            .when(en, |d| d.bg(theme.primary))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(en, |d| {
                                d.child(div().text_size(px(9.)).text_color(theme.primary_foreground).child("✓"))
                            }),
                    )
                    .child(div().text_size(px(12.)).child("Enabled")),
            );
            card = card.child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .mt_1()
                    .child(
                        div()
                            .id("autom-cancel")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.close_autom_dialog(cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("autom-save")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .text_color(theme.primary_foreground)
                            .bg(theme.primary)
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.submit_autom_dialog(cx);
                            }))
                            .child(if editing { "Save" } else { "Create automation" }),
                    ),
            );
            autom_overlay = autom_overlay
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _e, _w, cx| {
                                this.close_autom_dialog(cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    |_e, _w, cx| cx.stop_propagation(),
                                )
                                .child(card),
                        ),
                );
        }

        v_flex()
            .size_full()
            .key_context("native")
            .track_focus(&self.focus_handle)
            .id("native-root")
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_chord(ev, w, cx);
            }))
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _w, cx| {
                if this.split_dragging {
                    // Split area starts after the 51px rail plus the 257px
                    // sidebar card (hidden when collapsed); the changes dock
                    // (300px) takes from the right when open.
                    let mut x0 = 51.0_f32;
                    if !this.sidebar_collapsed
                        && !this.view_inbox
                        && !this.view_tasks
                        && !this.view_kanban
                    {
                        x0 += 257.0;
                    }
                    let right = if this.view_changes { 300.0_f32 } else { 0.0 };
                    let vw = f32::from(_w.viewport_size().width);
                    let avail = (vw - x0 - right).max(1.0);
                    let r = (f32::from(ev.position.x) - x0) / avail;
                    this.split_ratio = r.clamp(0.25, 0.75);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, _w, cx| {
                    if this.split_dragging {
                        this.split_dragging = false;
                        cx.notify();
                    }
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleFind, w, cx| {
                this.toggle_find(w, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleTerminal, w, cx| {
                this.toggle_terminal(w, cx);
            }))
            .bg(content_bg)
            .text_color(theme.foreground)
            .child(strip)
            .child(body)
            // Overlays mount only while active — an empty full-size overlay
            // would otherwise sit on top and eat every click.
            .when(
                self.ctx_menu.is_some()
                    || self.file_ctx_menu.is_some()
                    || self.proj_ctx_menu.is_some(),
                |d| d.child(ctx_overlay),
            )
            .when(self.hover_card.is_some(), |d| d.child(hc_overlay))
            .when(self.project_card.is_some(), |d| d.child(pc_overlay))
            .when(self.blame_pop.is_some(), |d| d.child(bp_overlay))
            .when(
                self.renaming || self.proj_rename.is_some(),
                |d| d.child(ren_overlay),
            )
            .when(self.palette_mode.is_some(), |d| d.child(palette_overlay))
            .when(self.sc_rec.is_some(), |d| d.child(sc_overlay))
            .when(self.feedback_open, |d| d.child(fb_overlay))
            .when(self.sc_dialog_open, |d| d.child(scd_overlay))
            .when(self.tour_step.is_some(), |d| d.child(tour_overlay))
            .when(self.git_dialog.is_some(), |d| d.child(git_overlay))
            .when(self.autom_dialog.is_some(), |d| d.child(autom_overlay))
    }
}
