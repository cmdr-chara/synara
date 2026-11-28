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
    slider::{Slider, SliderEvent, SliderState},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::component::WindowExt as _;
use gpui_kit::base::Disableable as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;

use crate::ws_client::Client;

actions!(native, [ToggleFind, ToggleTerminal, ToggleBrowser, ToggleComputer]);

mod browsercua;
mod composer;
mod dialogs;
mod explorer;
mod git;
mod keybindings;
mod overlays;
mod panes;
mod workspace;
mod palette;
mod projects;
mod settings;
mod snapshot;
mod terminal;
mod threads;
mod updater;
mod vault;
mod voice;

// Real upstream icon assets: Hugeicons glyphs regenerated as SVG, Central
// icons copied verbatim from apps/web/public/central-icons-reversed.
mod icons {
    pub const HOME: &[u8] = include_bytes!("../../assets/icons/home-07.svg");
    pub const INBOX: &[u8] = include_bytes!("../../assets/icons/inbox.svg");
    pub const SPACES: &[u8] = include_bytes!("../../assets/icons/folder-library.svg");
    pub const KANBAN: &[u8] = include_bytes!("../../assets/icons/columns-3-wide.svg");
    pub const TASKS: &[u8] = include_bytes!("../../assets/icons/apple-reminder.svg");
    pub const PULL_REQUESTS: &[u8] =
        include_bytes!("../../assets/icons/git-compare-arrows.svg");
    pub const EXTERNAL_LINK: &[u8] =
        include_bytes!("../../assets/icons/arrow-corner-right-up.svg");
    pub const MERGE_CONFLICT: &[u8] =
        include_bytes!("../../assets/icons/merge-conflict.svg");
    pub const PEOPLE_2: &[u8] = include_bytes!("../../assets/icons/people-2.svg");
    pub const BRANCH: &[u8] = include_bytes!("../../assets/icons/branch-simple.svg");
    pub const USER_GROUP: &[u8] = include_bytes!("../../assets/icons/user-group.svg");
    pub const BUBBLE_TEXT: &[u8] = include_bytes!("../../assets/icons/bubble-text.svg");
    pub const CHECK_CIRCLE: &[u8] =
        include_bytes!("../../assets/icons/circle-check.svg");
    pub const X_CIRCLE: &[u8] = include_bytes!("../../assets/icons/circle-x.svg");
    pub const LOADER: &[u8] = include_bytes!("../../assets/icons/loader.svg");
    pub const MERGED_SIMPLE: &[u8] =
        include_bytes!("../../assets/icons/merged-simple.svg");
    pub const PR_SIMPLE: &[u8] =
        include_bytes!("../../assets/icons/pull-request-simple.svg");
    pub const PR_CLOSED_SIMPLE: &[u8] =
        include_bytes!("../../assets/icons/pull-request-closed-simple.svg");
    pub const LINK: &[u8] = include_bytes!("../../assets/icons/chain-link-1.svg");
    pub const AUTOMATIONS: &[u8] = include_bytes!("../../assets/icons/clock-hour-7.svg");
    pub const STUDIO: &[u8] = include_bytes!("../../assets/icons/dashboard-circle.svg");
    pub const SIDEBAR_LEFT: &[u8] =
        include_bytes!("../../assets/icons/sidebar-simple-left-wide.svg");
    pub const SIDEBAR_RIGHT: &[u8] =
        include_bytes!("../../assets/icons/sidebar-simple-right-wide.svg");
    pub const SEARCH: &[u8] = include_bytes!("../../assets/icons/magnifying-glass.svg");
    pub const NEW_THREAD: &[u8] = include_bytes!("../../assets/icons/compose-pencil.svg");
    pub const BELL: &[u8] = include_bytes!("../../assets/icons/notes.svg");
    pub const USER: &[u8] = include_bytes!("../../assets/icons/user.svg");
    pub const HELP: &[u8] = include_bytes!("../../assets/icons/circle-questionmark.svg");
    pub const MORE: &[u8] = include_bytes!("../../assets/icons/dot-grid-1x3-horizontal.svg");
    pub const PLUS: &[u8] = include_bytes!("../../assets/icons/plus-sign.svg");
    pub const SEND: &[u8] = include_bytes!("../../assets/icons/arrow-up.svg");
    pub const MIC: &[u8] = include_bytes!("../../assets/icons/mic.svg");
    pub const ARROW_DOWN: &[u8] = include_bytes!("../../assets/icons/arrow-down.svg");
    pub const X: &[u8] = include_bytes!("../../assets/icons/x.svg");
    pub const UNDO: &[u8] = include_bytes!("../../assets/icons/arrow-undo-up.svg");
    pub const STEER: &[u8] = include_bytes!("../../assets/icons/corner-down-right.svg");
    pub const BOT: &[u8] = include_bytes!("../../assets/icons/robot-3.svg");
    pub const FILE_TEXT: &[u8] = include_bytes!("../../assets/icons/file-text.svg");
    pub const PAPERCLIP: &[u8] = include_bytes!("../../assets/icons/paperclip-1.svg");
    pub const WINDOW: &[u8] = include_bytes!("../../assets/icons/window.svg");
    pub const GOAL: &[u8] = include_bytes!("../../assets/icons/target-arrow.svg");
    pub const TODOS: &[u8] = include_bytes!("../../assets/icons/todos.svg");
    pub const BUG: &[u8] = include_bytes!("../../assets/icons/bug.svg");
    pub const ZAP: &[u8] = include_bytes!("../../assets/icons/zap.svg");
    pub const FLAG: &[u8] = include_bytes!("../../assets/icons/flag.svg");
    pub const CHECK: &[u8] = include_bytes!("../../assets/icons/checkmark-1.svg");
    pub const UNSTAGE: &[u8] =
        include_bytes!("../../assets/icons/arrow-rotate-counter-clockwise.svg");
    pub const STAGE: &[u8] = include_bytes!("../../assets/icons/plus-medium.svg");
    pub const FOLDER: &[u8] = include_bytes!("../../assets/icons/folder-closed.svg");
    pub const FOLDER_ADD: &[u8] = include_bytes!("../../assets/icons/folder-add.svg");
    pub const TEMPORARY: &[u8] =
        include_bytes!("../../assets/icons/bubble-annotation-5.svg");
    pub const DOWNLOAD: &[u8] = include_bytes!("../../assets/icons/download.svg");
    pub const IMPORT: &[u8] = include_bytes!("../../assets/icons/import.svg");
    pub const TERMINAL: &[u8] = include_bytes!("../../assets/icons/square-terminal.svg");
    pub const PIN: &[u8] = include_bytes!("../../assets/icons/pin.svg");
    pub const PANEL_TOP_OPEN: &[u8] =
        include_bytes!("../../assets/icons/panel-top-open.svg");
    pub const LAYOUT_RIGHT: &[u8] = include_bytes!("../../assets/icons/layout-right.svg");
    pub const PIN_FILL: &[u8] = include_bytes!("../../assets/icons/pin-fill.svg");
    pub const STAR: &[u8] = include_bytes!("../../assets/icons/star.svg");
    pub const STAR_FILLED: &[u8] = include_bytes!("../../assets/icons/star-filled.svg");
    pub const FOLDER_OPEN: &[u8] = include_bytes!("../../assets/icons/folder-open.svg");
    pub const CODEX: &[u8] = include_bytes!("../../assets/icons/codex.svg");
    pub const P_CLAUDE: &[u8] = include_bytes!("../../assets/icons/claude.svg");
    pub const P_CURSOR: &[u8] = include_bytes!("../../assets/icons/cursor.svg");
    pub const P_GROK: &[u8] = include_bytes!("../../assets/icons/grok.svg");
    pub const P_PI: &[u8] = include_bytes!("../../assets/icons/pi.svg");
    pub const P_OPENCODE: &[u8] = include_bytes!("../../assets/icons/opencode.svg");
    pub const P_DROID: &[u8] = include_bytes!("../../assets/icons/droid.svg");
    pub const P_DEVIN: &[u8] = include_bytes!("../../assets/icons/devin.svg");
    pub const P_OMP: &[u8] = include_bytes!("../../assets/icons/omp.svg");
    pub const P_ANTIGRAVITY: &[u8] = include_bytes!("../../assets/icons/antigravity.svg");
    pub const LOGO: &[u8] = include_bytes!("../../assets/icons/synara-logo.svg");
    pub const LOGO_MOTION: &[u8] = include_bytes!("../../assets/icons/synara-logo-motion.svg");
    pub const SHIELD_ACCESS: &[u8] =
        include_bytes!("../../assets/icons/shield-access.svg");
    pub const SHIELD_CODE: &[u8] = include_bytes!("../../assets/icons/shield-code.svg");
    pub const CHEVRON_DOWN: &[u8] =
        include_bytes!("../../assets/icons/chevron-down-small.svg");
    pub const CHEVRON_LEFT: &[u8] =
        include_bytes!("../../assets/icons/chevron-left.svg");
    pub const CHEVRON_RIGHT: &[u8] =
        include_bytes!("../../assets/icons/chevron-right.svg");
    pub const WORKTREE: &[u8] =
        include_bytes!("../../assets/icons/worktree-split.svg");
    pub const REFRESH: &[u8] =
        include_bytes!("../../assets/icons/arrow-rotate-clockwise.svg");
    pub const SORT: &[u8] =
        include_bytes!("../../assets/icons/sort-arrow-up-down.svg");
    pub const FILTER: &[u8] = include_bytes!("../../assets/icons/filter-1.svg");
    // Code review header (upstream central-icons arrow-top-bottom / filter-2).
    pub const ARROW_TOP_BOTTOM: &[u8] = include_bytes!("../../assets/icons/arrow-top-bottom.svg");
    pub const FILTER_2: &[u8] = include_bytes!("../../assets/icons/filter-2.svg");
    pub const PLUS_MED: &[u8] = include_bytes!("../../assets/icons/plus-medium.svg");
    pub const PENCIL: &[u8] = include_bytes!("../../assets/icons/pencil.svg");
    pub const TRASH: &[u8] = include_bytes!("../../assets/icons/trash-can.svg");
    pub const S_PEOPLE_ID_CARD: &[u8] =
        include_bytes!("../../assets/icons/people-id-card.svg");
    pub const S_COLOR_PALETTE: &[u8] =
        include_bytes!("../../assets/icons/color-palette.svg");
    pub const S_LAYOUT_SIDEBAR: &[u8] =
        include_bytes!("../../assets/icons/layout-sidebar.svg");
    pub const S_CHAT_BUBBLES: &[u8] =
        include_bytes!("../../assets/icons/chat-bubbles.svg");
    pub const S_BELL: &[u8] = include_bytes!("../../assets/icons/bell.svg");
    pub const S_SHORTCUT: &[u8] = include_bytes!("../../assets/icons/shortcut.svg");
    pub const S_PUZZLE: &[u8] = include_bytes!("../../assets/icons/puzzle.svg");
    pub const S_BRAIN: &[u8] = include_bytes!("../../assets/icons/brain.svg");
    pub const S_BUILDING_BLOCKS: &[u8] =
        include_bytes!("../../assets/icons/building-blocks.svg");
    pub const S_GAUGE: &[u8] = include_bytes!("../../assets/icons/gauge.svg");
    pub const S_CONNECTORS: &[u8] =
        include_bytes!("../../assets/icons/connectors-1.svg");
    pub const S_PLUGIN: &[u8] = include_bytes!("../../assets/icons/plugin-1.svg");
    pub const S_GLOBE: &[u8] = include_bytes!("../../assets/icons/globe.svg");
    pub const S_COMPUTER_USE: &[u8] =
        include_bytes!("../../assets/icons/computer-use.svg");
    pub const S_SCREEN_CAPTURE: &[u8] =
        include_bytes!("../../assets/icons/screen-capture.svg");
    pub const S_BRANCH_SIMPLE: &[u8] =
        include_bytes!("../../assets/icons/branch-simple.svg");
    pub const S_ARCHIVE: &[u8] = include_bytes!("../../assets/icons/archive.svg");
    pub const S_SHIELD: &[u8] = include_bytes!("../../assets/icons/shield.svg");
    pub const HANDOFF: &[u8] = include_bytes!("../../assets/icons/handoff.svg");
    pub const COPY: &[u8] = include_bytes!("../../assets/icons/copy.svg");
    pub const TEXT_WRAP: &[u8] = include_bytes!("../../assets/icons/text-wrap.svg");
    pub const BRANCH_SIMPLE: &[u8] = include_bytes!("../../assets/icons/branch-simple.svg");
    pub const GIT_COMMIT: &[u8] = include_bytes!("../../assets/icons/commits.svg");
    pub const ROBOT: &[u8] = include_bytes!("../../assets/icons/robot-3.svg");
    pub const DOWN_WALL: &[u8] = include_bytes!("../../assets/icons/arrow-down-wall.svg");
    pub const SIDECHAT: &[u8] = include_bytes!("../../assets/icons/sidechat.svg");
    pub const CLOCK: &[u8] = include_bytes!("../../assets/icons/clock.svg");
    pub const S_TOOLBOX: &[u8] = include_bytes!("../../assets/icons/toolbox.svg");
    pub const S_CIRCLE_INFO: &[u8] =
        include_bytes!("../../assets/icons/circle-info.svg");
    pub const S_GEAR: &[u8] =
        include_bytes!("../../assets/icons/settings-gear-4.svg");
    pub const B_SUPABASE: &[u8] = include_bytes!("../../assets/icons/brands/supabase.svg");
    pub const B_VERCEL: &[u8] = include_bytes!("../../assets/icons/brands/vercel.svg");
    pub const B_SLACK: &[u8] = include_bytes!("../../assets/icons/brands/slack.svg");
    pub const B_LINEAR: &[u8] = include_bytes!("../../assets/icons/brands/linear.svg");
    pub const B_NOTION: &[u8] = include_bytes!("../../assets/icons/brands/notion.svg");
    pub const B_STRIPE: &[u8] = include_bytes!("../../assets/icons/brands/stripe.svg");
    pub const B_WORKOS: &[u8] = include_bytes!("../../assets/icons/brands/workos.svg");
    pub const B_CONVEX: &[u8] = include_bytes!("../../assets/icons/brands/convex.svg");
    pub const B_CLOUDFLARE: &[u8] = include_bytes!("../../assets/icons/brands/cloudflare.svg");
    pub const B_SENTRY: &[u8] = include_bytes!("../../assets/icons/brands/sentry.svg");
    pub const B_NEON: &[u8] = include_bytes!("../../assets/icons/brands/neon.svg");
    pub const B_POSTHOG: &[u8] = include_bytes!("../../assets/icons/brands/posthog.svg");
    pub const B_NETLIFY: &[u8] = include_bytes!("../../assets/icons/brands/netlify.svg");
    pub const B_RAILWAY: &[u8] = include_bytes!("../../assets/icons/brands/railway.svg");
    pub const B_PRISMA: &[u8] = include_bytes!("../../assets/icons/brands/prisma.svg");
    pub const B_HUGGINGFACE: &[u8] = include_bytes!("../../assets/icons/brands/huggingface.svg");
    pub const B_ATLASSIAN: &[u8] = include_bytes!("../../assets/icons/brands/atlassian.svg");
    pub const B_TODOIST: &[u8] = include_bytes!("../../assets/icons/brands/todoist.svg");
    pub const B_MCP: &[u8] = include_bytes!("../../assets/icons/brands/mcp.svg");
}

/// Transcript/PR markdown style: follows the active light/dark theme instead
/// of the hardcoded dark highlighter the first pass used. Upstream spec:
/// code surface radius 16px, padding 8px 9.6px — the gpui-component theme
/// already derives radius/padding/background; only the top inset for the
/// actions row and the highlight theme need setting here.
fn markdown_style(dark: bool) -> gpui_kit::component::text::TextViewStyle {
    let mut s = gpui_kit::component::text::TextViewStyle::default();
    s.code_block.padding.top = Some(px(28.).into());
    // Upstream: code surface follows the user-message token (3% ink in light),
    // radius 0.65rem = 10.4px.
    s.code_block.background = Some(if dark {
        gpui::hsla(0.0, 0.0, 1.0, 0.06).into()
    } else {
        gpui::hsla(0.0, 0.0, 0.0, 0.03).into()
    });
    s.code_block.corner_radii.top_left = Some(px(10.4).into());
    s.code_block.corner_radii.top_right = Some(px(10.4).into());
    s.code_block.corner_radii.bottom_left = Some(px(10.4).into());
    s.code_block.corner_radii.bottom_right = Some(px(10.4).into());
    s.highlight_theme = if dark {
        gpui_kit::component::highlighter::HighlightTheme::default_dark()
    } else {
        gpui_kit::component::highlighter::HighlightTheme::default_light()
    };
    s.is_dark = dark;
    s
}

// Latest context-window usage emitted by the runtime, scanning activities
// backwards (upstream deriveLatestContextWindowState): a fresh configuration
// or a completed compaction invalidates older usage. Returns
// (used_percentage 0..100 clamped, used_tokens, max_tokens).
fn context_window_usage(thread_v: &Value) -> Option<(f64, f64, Option<f64>)> {
    for a in arr(thread_v, "activities").iter().rev() {
        let kind = str_at(a, "kind");
        if kind == "context-window.configured" {
            return None;
        }
        if kind == "context-compaction" {
            let st = str_at(a, "payload.state");
            if st == "compacted" || str_at(a, "payload.status") == "completed" {
                return None;
            }
        }
        if kind != "context-window.updated" {
            continue;
        }
        let used = a.pointer("/payload/usedTokens").and_then(Value::as_f64).unwrap_or(0.0);
        let pct = a.pointer("/payload/usedPercent").and_then(Value::as_f64).map(|p| p.clamp(0.0, 100.0));
        let max_t = a.pointer("/payload/maxTokens").and_then(Value::as_f64).filter(|m| *m > 0.0);
        if used <= 0.0 && pct.is_none() && max_t.is_none() {
            continue;
        }
        let used_pct = pct.or_else(|| max_t.map(|m| (used / m * 100.0).min(100.0)))?;
        return Some((used_pct, used, max_t));
    }
    None
}

// Ring svg for the compact context meter (upstream ContextWindowMeter:
// 16px, r=6, 2px stroke, dash-offset arc). Cached per integer percent —
// svg data needs a static lifetime, so strings leak once per value.
fn meter_ring_data(pct: f64) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<u32, &'static str>>> = OnceLock::new();
    let key = pct.round().clamp(0.0, 100.0) as u32;
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap();
    guard.entry(key).or_insert_with(|| {
        let r = 6.0f64;
        let circ = 2.0 * std::f64::consts::PI * r;
        let offset = circ - (key as f64 / 100.0) * circ;
        Box::leak(format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'>\
<circle cx='8' cy='8' r='6' fill='none' stroke='currentColor' stroke-opacity='0.25' stroke-width='2'/>\
<circle cx='8' cy='8' r='6' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' \
stroke-dasharray='{circ:.3}' stroke-dashoffset='{offset:.3}' transform='rotate(-90 8 8)'/></svg>"
        ).into_boxed_str())
    })
}

fn svg_icon(data: &'static [u8]) -> Icon {
    Icon::default().data(data)
}

/// Live-work brand mark: the Synara outline draws itself in, then the wings
/// lengthen inward until the negative space at the center is filled.
fn animated_logo(icon_size: Pixels, color: gpui::Hsla) -> impl gpui::Element {
    gpui::svg()
        .data(icons::LOGO_MOTION)
        .size(icon_size)
        .text_color(color)
        .with_animation(
            "synara-working-logo-extend",
            gpui::Animation::new(std::time::Duration::from_millis(2400)).repeat_synced(),
            |this, delta| {
                let smooth = |t: f32| {
                    let t = t.clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                };
                let extend = smooth((delta - 0.34) / 0.34);
                let scale = 1.0 + 0.22 * extend;
                let offset = if delta < 0.34 {
                    100.0 * (1.0 - delta / 0.34)
                } else {
                    0.0
                };
                let stroke_alpha = if delta < 0.42 {
                    1.0
                } else if delta < 0.58 {
                    1.0 - (delta - 0.42) / 0.16
                } else {
                    0.0
                };
                let fill_alpha = smooth((delta - 0.18) / 0.25)
                    * (1.0 - smooth((delta - 0.86) / 0.14));
                let wing_transform = |left: bool| {
                    let (pivot_x, angle) = if left { (0.0, 57.0) } else { (470.0, 123.0) };
                    format!(
                        "translate({:.1} 0) rotate({:.1}) scale({:.3} 1) rotate({:.1}) translate({:.1} 0)",
                        pivot_x,
                        angle,
                        scale,
                        -angle,
                        -pivot_x,
                    )
                };
                let frame = String::from_utf8_lossy(icons::LOGO_MOTION)
                    .replace("__OFFSET__", &format!("{offset:.2}"))
                    .replace("__STROKE_ALPHA__", &format!("{stroke_alpha:.3}"))
                    .replace("__FILL_ALPHA__", &format!("{fill_alpha:.3}"))
                    .replace("__LEFT_TRANSFORM__", &wing_transform(true))
                    .replace("__RIGHT_TRANSFORM__", &wing_transform(false));
                this.data(frame.as_bytes())
            },
        )
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

/// Upstream deriveActiveTaskListState: the latest `turn.tasks.updated`
/// snapshot wins — the current turn's non-empty list, else the newest
/// unfinished prior-turn list. Empty snapshots clear the card.
fn derive_active_task_list(thread_v: &Value) -> Option<Vec<(String, u8)>> {
    let latest_turn_id = str_at(get(thread_v, "latestTurn"), "turnId").to_string();
    let lists: Vec<(String, Vec<(String, u8)>)> = arr(thread_v, "activities")
        .iter()
        .filter(|a| str_at(a, "kind") == "turn.tasks.updated")
        .filter_map(|a| {
            let payload = get(a, "payload");
            if !get(payload, "tasks").is_array() {
                return None;
            }
            let tasks: Vec<(String, u8)> = arr(payload, "tasks")
                .iter()
                .filter_map(|t| {
                    let task = str_at(t, "task").to_string();
                    if task.is_empty() {
                        return None;
                    }
                    let status = match str_at(t, "status") {
                        "inProgress" => 0u8,
                        "completed" => 2u8,
                        _ => 1u8,
                    };
                    Some((task, status))
                })
                .collect();
            Some((str_at(a, "turnId").to_string(), tasks))
        })
        .collect();
    if !latest_turn_id.is_empty() {
        if let Some((_, tasks)) = lists.iter().rev().find(|(t, _)| *t == latest_turn_id) {
            return (!tasks.is_empty()).then(|| tasks.clone());
        }
    }
    let (_, tasks) = lists.last()?;
    if tasks.is_empty() || tasks.iter().all(|(_, s)| *s == 2) {
        return None;
    }
    Some(tasks.clone())
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

// ── GitHub inbox list helpers (upstream githubInbox.logic.ts +
// pullRequestList.logic.ts, reduced to the native filter surface) ─────────

/// Upstream formatRelativeTime compact English labels: now, 5m, 3h, 2d, 1w, 5mo.
fn gh_relative_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let diff = chrono::Utc::now().signed_duration_since(t.with_timezone(&chrono::Utc));
    let minutes = diff.num_minutes().max(0);
    if minutes < 1 {
        return "now".to_string();
    }
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 7 {
        return format!("{days}d");
    }
    if days < 30 {
        return format!("{}w", days / 7);
    }
    if days < 365 {
        return format!("{}mo", days / 30);
    }
    format!("{}y", days / 365)
}

fn gh_item_matches_query(item: &Value, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return true;
    }
    let labels = arr(item, "labels")
        .iter()
        .map(|l| str_at(l, "name"))
        .collect::<Vec<_>>()
        .join(" ");
    let haystack = format!(
        "{} {} {} #{} {} {}",
        str_at(item, "title"),
        str_at(item, "repository"),
        str_at(item, "headBranch"),
        get(item, "number").as_i64().unwrap_or(0),
        str_at(get(item, "author"), "login"),
        labels,
    )
    .to_lowercase();
    haystack.contains(normalized_query)
}

/// Rows after the native kind/state/text filters. `merged` narrows the
/// closed superset to merged pull requests, as upstream.
fn gh_visible_items(
    items: &[Value],
    kind: &str,
    state_filter: &str,
    query: &str,
    sort: &str,
) -> Vec<Value> {
    let q = query.trim().to_lowercase();
    let stamp = if sort == "updated" { "updatedAt" } else { "createdAt" };
    let mut rows: Vec<Value> = items
        .iter()
        .filter(|item| kind == "all" || str_at(item, "kind") == kind)
        .filter(|item| {
            state_filter != "merged"
                || (str_at(item, "kind") == "pullRequest" && str_at(item, "state") == "merged")
        })
        .filter(|item| gh_item_matches_query(item, &q))
        .cloned()
        .collect();
    rows.sort_by(|a, b| {
        str_at(b, stamp)
            .cmp(str_at(a, stamp))
            .then(
                get(b, "number")
                    .as_i64()
                    .unwrap_or(0)
                    .cmp(&get(a, "number").as_i64().unwrap_or(0)),
            )
    });
    // Pinned rows lead even inside the flat All section; sectioning happens at render time.
    rows.sort_by_key(|item| if get(item, "isPinned").as_bool() == Some(true) { 0 } else { 1 });
    rows
}

fn gh_actor_label(item: &Value) -> String {
    let author = get(item, "author");
    let name = str_at(author, "name").trim();
    if !name.is_empty() {
        return name.to_string();
    }
    let login = str_at(author, "login");
    if login.is_empty() {
        "unknown".to_string()
    } else {
        login.to_string()
    }
}

fn icon_btn(id: &'static str, data: &'static [u8], size: f32, hover: Hsla, ink: Hsla) -> Stateful<Div> {
    icon_btn_pad(id, data, size, 16., hover, ink)
}

/// Upstream ProviderIcon at 16px: Claude, Antigravity and Oh My Pi are
/// full-colour marks; the rest render the mono glyph in `ink`.
fn provider_icon_16(pid: &str, mono: &'static [u8], ink: Hsla) -> gpui::AnyElement {
    let file = match pid {
        "claudeAgent" | "claude" => Some("claude"),
        "antigravity" => Some("antigravity-color"),
        "omp" => Some("omp"),
        _ => None,
    };
    match file {
        Some(file) => gpui::img(std::path::PathBuf::from(format!(
            "{}/assets/icons/{file}.svg",
            env!("CARGO_MANIFEST_DIR")
        )))
        .size(px(16.))
        .into_any_element(),
        None => svg_icon(mono).size(px(16.)).text_color(ink).into_any_element(),
    }
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

/// Import projects dialog state (upstream ProjectImportDialog +
/// ProjectImportPanel): chosen provider sources, the scanned catalog,
/// per-item selection and run progress. `stop` is shared with the async
/// import loop so "Stop after current" halts between items.
struct ImportDlg {
    providers: Vec<String>,
    catalog: Option<Value>,
    scanning: bool,
    running: bool,
    error: Option<String>,
    selected: std::collections::HashSet<String>,
    roots: std::collections::HashMap<String, String>,
    include_archived: bool,
    search: Entity<InputState>,
    outcomes: std::collections::HashMap<String, Option<String>>,
    progress: Option<(usize, usize, String)>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    expanded: std::collections::HashSet<String>,
    root_inputs: std::collections::HashMap<String, Entity<InputState>>,
}

/// projectImportItemKey: JSON [projectKey, threadKey|null].
fn import_item_key(project_key: &str, thread_key: Option<&str>) -> String {
    match thread_key {
        Some(tk) => json!([project_key, tk]).to_string(),
        None => json!([project_key, Value::Null]).to_string(),
    }
}

/// selectableProjectImportKeys: a threadless project is selectable only
/// when it would create a new project; threads skip imported and,
/// unless enabled, archived items.
fn import_selectable_keys(project: &Value, include_archived: bool) -> Vec<String> {
    let pk = str_at(project, "key");
    let threads = arr(project, "threads");
    if threads.is_empty() {
        return if get(project, "existingProjectId").is_null() {
            vec![import_item_key(pk, None)]
        } else {
            Vec::new()
        };
    }
    threads
        .iter()
        .filter(|t| {
            !t["alreadyImported"].as_bool().unwrap_or(false)
                && (include_archived || !t["archived"].as_bool().unwrap_or(false))
        })
        .map(|t| import_item_key(pk, Some(str_at(t, "key"))))
        .collect()
}

/// Import providers use kind names ("claudeAgent"); map them onto the
/// brand icons before falling back to the session provider map.
fn import_provider_icon(provider: &str) -> &'static [u8] {
    match provider {
        "claudeAgent" => icons::P_CLAUDE,
        "codex" => icons::CODEX,
        other => provider_icon(other),
    }
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
static SHORTCUT_ROWS: [(&str, &str, &str, &str); 70] = [
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
    ("Toggle computer", "Reveal the computer-use panel: live desktop stream and control.", "", "computer.toggle"),
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
    /// "Show more" paging per sidebar list (upstream THREAD_PREVIEW_PAGE_SIZE=5):
    /// number of extra revealed pages, keyed by project id ("chats" for the
    /// Chats group, "pinned" for the Pinned section).
    preview_pages: std::collections::HashMap<String, u32>,
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
    inbox_recap_prev: Option<Value>,
    inbox_usage: Vec<Value>,
    inbox_signed_in: Option<bool>,
    inbox_updates: Vec<Value>,
    tools_expanded: bool,
    subagents_expanded: bool,
    pending_attachments: Vec<Value>,
    attachment_thumbs: std::collections::HashMap<String, std::sync::Arc<RenderImage>>,
    composer_thread_id: Option<String>,
    uploading: bool,
    interaction_mode: &'static str,
    fast_mode: bool,
    /// Composer model selection: (provider, model slug, display label).
    composer_model: (String, String, String),
    /// Model picker panel open (upstream ComposerModelPicker popup).
    model_picker_open: bool,
    /// Search input inside the model picker panel.
    model_picker_search: Entity<InputState>,
    /// Catalog rows (slug, display name, provider) from provider.listModels,
    /// empty until the first successful fetch.
    model_catalog: Vec<(String, String, String)>,
    /// Effort ladder index for the picker slider (upstream trait effort).
    composer_effort_idx: usize,
    /// Provider tab open in the model picker (upstream ComposerModelPickerTab).
    model_picker_provider: String,
    queued_turns: Vec<String>,
    queue_paused: bool,
    /// Debug task-list popover open (upstream ComposerCapsulePopover state).
    task_card_open: bool,
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
    backdrop_full: bool,
    /// AsciiWaveBackground (upstream): ambient field clock. `ascii_cells`
    /// holds the per-cell glyph indices so scrambling is incremental, and
    /// `ascii_ripples` the live click ripples (x, y, phaseA, phaseB, start).
    ascii_t0: std::time::Instant,
    ascii_seed: f32,
    ascii_cells: Vec<u8>,
    /// Dithered temple backdrop (upstream EmptyChatBackdrop): cached rendered
    /// dot field plus the (w,h,dark) key it was built at.
    backdrop_img: Option<Arc<gpui::RenderImage>>,
    backdrop_field: Option<crate::backdrop::BackdropField>,
    backdrop_spec: Option<crate::backdrop::BackdropSpec>,
    backdrop_key: (u32, u32, bool),
    /// Full-window atmosphere layer (upstream .app-shell-atmosphere):
    /// window-coordinated texture composited at 35%/80% opacity under the
    /// landing texture, exactly like the fork's two-canvas structure.
    atmosphere_img: Option<Arc<gpui::RenderImage>>,
    atmosphere_key: (u32, u32, bool, u32),
    ascii_ripples: Vec<(f32, f32, f32, f32, std::time::Instant)>,
    /// Cell pitch measured like upstream (`measureText("M").width + 1` in the
    /// UI font); 0 until first measured.
    ascii_cell_w: f32,
    /// Set when the landing leaves the screen; the next landing mounts a
    /// fresh field (upstream remounts AsciiWaveBackground).
    ascii_fresh: bool,
    renaming: bool,
    rename_input: Option<Entity<InputState>>,
    view_changes: bool,
    staged_files: Vec<Value>,
    unstaged_files: Vec<Value>,
    changes_diff: Option<(String, String)>,
    changes_diff_state: Option<Entity<gpui_kit::component::diff::DiffState>>,
    // Code review page list state (upstream GitHubInbox): the native
    // `githubInbox.list` superset, its filters, and the search field.
    gh_items: Vec<Value>,
    gh_errors: Vec<Value>,
    gh_viewer: String,
    gh_loading: bool,
    gh_epoch: u64,
    gh_kind: &'static str,
    gh_state_filter: &'static str,
    gh_sort: &'static str,
    gh_search: Entity<InputState>,
    // Code review page detail pane (upstream PullRequestDetailPanel):
    // the selected PR, its detail payload, the active tab and the Code
    // tab's diff viewport state.
    pr_sel: Option<(String, String, u64)>,
    issue_sel: Option<(String, String, u64)>,
    issue_detail: Option<Value>,
    issue_detail_loading: bool,
    issue_detail_tab: usize,
    issue_detail_epoch: u64,
    issue_comment_input: Option<Entity<gpui_kit::component::input::TextareaState>>,
    issue_comment_posting: bool,
    pr_detail: Option<Value>,
    pr_detail_loading: bool,
    pr_detail_tab: usize,
    pr_detail_epoch: u64,
    pr_diff_state: Option<Entity<gpui_kit::component::diff::DiffState>>,
    pr_diff_loading: bool,
    pr_diff_error: Option<String>,
    pr_diff_truncated: bool,
    pr_disc: [bool; 3],
    pr_comment_input: Option<Entity<gpui_kit::component::input::TextareaState>>,
    /// Anchored "…" actions menu of the pull request detail header (x, y).
    pr_actions_menu: Option<(f32, f32)>,
    /// Stack popover (upstream PullRequestStackPopover) anchored at (x, y).
    pr_stack_menu: Option<(f32, f32)>,
    /// Irreversible action awaiting confirmation ("merge" | "close").
    pr_confirm: Option<&'static str>,
    /// Merge method picked in the actions menu radio group.
    pr_merge_method: String,
    /// Comment ids the reader collapsed in the Comments disclosure.
    pr_comments_closed: std::collections::HashSet<String>,
    /// Per-file diff stats of the loaded pull request patch (Code tab stat line).
    pr_diff_files: Vec<Value>,
    pr_summary_scroll: ScrollHandle,
    issue_scroll: ScrollHandle,
    pr_timeline_scroll: ScrollHandle,
    pr_comment_posting: bool,
    /// Diff-line blame popover (upstream DiffLineBlamePopover):
    /// (`git.blameLine` result, x, y).
    blame_pop: Option<(Value, f32, f32)>,
    changes_loading: bool,
    changes_unavailable: bool,
    // Right dock pane tabs (upstream RightDock/SurfaceContentTabs):
    // 0 = Git, 1 = Subagents, 2 = Background tasks, 3 = Side chats.
    dock_tab: usize,
    view_terminal: bool,
    /// Browser panel: embedded webview for dev-server preview.
    view_browser: bool,
    browser_webview: Option<Entity<gpui_webview::WebView>>,
    browser_input: Option<Entity<InputState>>,
    /// Computer-use panel: live desktop stream + control affordances.
    view_cua: bool,
    cua: Option<crate::cua::CuaHandle>,
    cua_seen_seq: u64,
    /// Logical screen points from computer.getScreenSize — the click
    /// mapping divides the PNG pixels by this, never assuming retina.
    cua_screen_points: Option<(f64, f64)>,
    /// Keyboard forwarding input inside the computer panel: typed text goes
    /// to the desktop as computer.typeText on Enter, then a Return key.
    cua_input: Option<Entity<InputState>>,
    term_active: bool,
    // TerminalWorkspaceTabs: one PTY session per tab; events route by id.
    term_tabs: Vec<TermTab>,
    term_sel: usize,
    settings_scroll: ScrollHandle,
    settings_nav_scroll: ScrollHandle,
    dock_tabs_scroll: ScrollHandle,
    settings_data: Value,
    settings_loading: bool,
    /// Owner-side browser credential vault surface (Saved logins). Snapshots
    /// carry metadata only; revealed secrets stay in `vault_revealed` briefly
    /// and never enter `settings_data` or diagnostics.
    vault_snapshot: Option<Value>,
    vault_error: Option<String>,
    vault_master_input: Entity<InputState>,
    vault_confirm_input: Entity<InputState>,
    vault_reveal_input: Entity<InputState>,
    vault_reveal_id: Option<String>,
    vault_revealed: Option<(String, String)>,
    vault_delete_id: Option<String>,
    vault_prompts_seen: std::collections::HashSet<String>,
    /// Owner cookie-import draft/result. Browser/profile choices are explicit;
    /// cookie values never pass through this UI state.
    cookie_sources: Vec<Value>,
    cookie_browser: Option<String>,
    cookie_profile: Option<String>,
    cookie_domain_input: Entity<InputState>,
    cookie_importing: bool,
    cookie_profile_confirmed: bool,
    cookie_result: Option<String>,
    cookie_error: Option<String>,
    /// Upstream dismissedThreadStatusKeyByThreadId (thread_id -> dismissalKey).
    dismissed_status: std::collections::HashMap<String, String>,
    /// Upstream lastVisitedAtByThreadId (thread_id -> ISO timestamp).
    visited_threads: std::collections::HashMap<String, String>,
    /// Upstream keepAwakeMode setting: 0 on, 1 agent, 2 off.
    keep_awake_mode: usize,
    notification_primed: bool,
    notification_states: std::collections::HashMap<String, (bool, bool)>,
    notification_keys: std::collections::HashSet<String>,
    notification_window: Option<gpui::AnyWindowHandle>,
    notification_window_active: bool,
    desktop_update_state: Option<crate::updater::UpdateState>,
    desktop_update_installing: bool,
    beta_action_running: Option<&'static str>,
    beta_action_message: Option<String>,
    /// Upstream OneTimeCoachmark: seen keys persisted under
    /// settings.sidebarUi.coachmarks; `coachmark` is the one showing.
    coachmarks_seen: std::collections::HashSet<String>,
    // Upstream DebugFeatureFlagsMenu: [persist-toasts, pin-git-progress,
    // show-debug-task-banner]; sidebar footer popup, dev-only surface.
    feature_flags: [bool; 3],
    flags_menu_open: bool,
    /// Home-landing "Work in a project" picker (upstream ProjectPicker):
    /// popover open state, the project the next thread draft targets, and the
    /// popover search input (created lazily on open).
    project_picker_open: bool,
    landing_project_id: Option<String>,
    /// Composer "Temporary" toggle for the empty landing (upstream
    /// draftThreadContext.isTemporary; the marker stays client-side).
    landing_temporary: bool,
    project_picker_input: Option<Entity<InputState>>,
    /// Upstream ThreadErrorBanner: thread ids whose session error was
    /// dismissed this session, the Show-details toggle for the current
    /// banner text, and the in-flight unblock flag.
    dismissed_thread_errors: std::collections::HashSet<String>,
    error_details_open: bool,
    error_details_for: Option<String>,
    unblocking_thread: bool,
    // Upstream ProviderHealthBanner: `server.getConfig.providers` statuses
    // plus the session-local dismissal keys (upstream persists them in
    // localStorage, capped at 50).
    provider_statuses: Vec<Value>,
    dismissed_provider_banner_keys: std::collections::HashSet<String>,
    plan_expanded: std::collections::HashSet<String>,
    // ExpiredSidechatNotice start-new in flight.
    starting_sidechat: bool,
    // Upstream PullRequestThreadDialog (checkout mode): resolve a GitHub PR
    // reference, preview it, then check it out into a new thread.
    pr_dialog_open: bool,
    pr_dialog_input: Option<Entity<InputState>>,
    pr_resolved: Option<Value>,
    pr_resolving: bool,
    pr_dialog_error: Option<String>,
    pr_busy: bool,
    pr_resolve_epoch: usize,
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
    md_images: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<RenderImage>>>,
    >,
    md_images_pending: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    activity_open: Vec<String>,
    turn_files_collapsed: Vec<String>,
    /// Live voice-note capture while the composer mic is recording.
    voice_capture: Option<crate::voice::VoiceCapture>,
    /// True while a finished clip is with the transcription backend.
    voice_transcribing: bool,
    /// voiceEnterBehavior "send": send the message once the transcription
    /// lands in the composer.
    voice_send_after: bool,
    ctx_menu: Option<(String, f32, f32)>,
    // True when ctx_menu was opened from an open-thread tab: upstream appends
    // "close tabs" scope rows on top of the usual thread menu.
    ctx_menu_tab: bool,
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
    hover_user_msg: Option<String>,
    project_card: Option<(String, f32, f32)>,
    project_card_seq: u64,
    /// Project rename dialog (upstream EditProjectDialog): id + input entity.
    proj_rename: Option<String>,
    proj_rename_input: Option<Entity<InputState>>,
    /// RelocateProjectDialog: (project id, previous workspace root).
    proj_relocate: Option<(String, String)>,
    proj_relocate_input: Option<Entity<InputState>>,
    /// Import projects dialog (upstream ProjectImportDialog).
    import_dlg: Option<ImportDlg>,
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
    /// Custom snooze date-time dialog (upstream SnoozeUntilDialog).
    snooze_dlg_tid: Option<String>,
    snooze_dlg_input: Option<Entity<InputState>>,
    /// App tour walkthrough (upstream AppTour): current step index.
    tour_step: Option<usize>,
    user_input_draft: Vec<(String, usize, String)>,
    // Blocking pending userInput prompt (upstream UserInputQuestionForm +
    // ComposerPendingUserInputPanel): drafts keyed requestId → questionId,
    // the active question index per request, and an in-flight respond flag.
    pi_drafts: std::collections::HashMap<String, std::collections::HashMap<String, PiDraft>>,
    pi_qidx: std::collections::HashMap<String, usize>,
    pi_responding: bool,
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
    /// Upstream useWhatsNew: bottom-left popout card on a covered upgrade.
    wn_popout: bool,
    /// What's-new dialog view: 0 closed, 1 current release, 2 changelog.
    wn_view: u8,
    wn_changelog_open: std::collections::HashSet<String>,
    /// settings.sidebarUi.whatsNewLastSeenVersion (NullOr sentinel: None =
    /// never dismissed, treated as fresh install on first launch).
    wn_seen: Option<String>,
    /// Per-variant theme pack overrides (accent/background/foreground hexes),
    /// persisted under settings.appearance.themePacks (upstream ThemePackEditor).
    theme_packs: Value,
    tp_edit: Option<(bool, String)>,
    tp_input: Option<Entity<InputState>>,
    /// Import dialog for a theme pack: (dark, multiline JSON input).
    tp_import: Option<bool>,
    /// Per-variant slider states for the theme pack editor (upstream
    /// ThemePackEditor's opacity/blur/contrast rows). Keys:
    /// "dark-opacity" | "dark-blur" | "dark-contrast" and "light-*".
    tp_sliders: std::collections::HashMap<String, Entity<SliderState>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GitDialog {
    Commit,
    Pr,
}

/// Draft answer to one blocking userInput question (upstream
/// PendingUserInputDraftAnswer): option labels selected and/or a custom
/// free-text answer, the custom text winning when both exist.
#[derive(Default, Clone)]
struct PiDraft {
    options: Vec<String>,
    custom: String,
}

/// The first actionable blocking userInput prompt of a thread detail
/// (upstream `pendingUserInputs[0]`): request id, optional lifecycle
/// generation, and the provider-authored questions parsed out of the newest
/// matching `user-input.requested` activity
/// (`derivePendingUserInputs` + `parseUserInputQuestions`).
fn active_pending_input(
    thread_v: &Value,
) -> Option<(String, Option<String>, Vec<Value>)> {
    for p in arr(thread_v, "pendingInteractions") {
        if str_at(p, "interactionKind") != "userInput" || str_at(p, "status") != "pending" {
            continue;
        }
        let rid = str_at(p, "requestId").to_string();
        if rid.is_empty() {
            continue;
        }
        let gen = {
            let g = str_at(p, "lifecycleGeneration").to_string();
            (!g.is_empty()).then_some(g)
        };
        let mut questions: Vec<Value> = Vec::new();
        for a in arr(thread_v, "activities").iter().rev() {
            if str_at(a, "kind") != "user-input.requested" {
                continue;
            }
            let payload = get(a, "payload");
            if str_at(payload, "requestId") != rid {
                continue;
            }
            questions = arr(payload, "questions")
                .iter()
                .filter(|q| {
                    !str_at(q, "id").is_empty()
                        && !str_at(q, "header").is_empty()
                        && !str_at(q, "question").is_empty()
                        && get(q, "options").is_array()
                })
                .cloned()
                .collect();
            if !questions.is_empty() {
                break;
            }
        }
        if !questions.is_empty() {
            return Some((rid, gen, questions));
        }
    }
    None
}

/// Resolved answer for one question (upstream resolvePendingUserInputAnswer):
/// the custom answer, the selected list for multi-select, or the first label.
fn pi_resolved(q: &Value, draft: Option<&PiDraft>) -> Option<Value> {
    let draft = draft?;
    let custom = draft.custom.trim();
    if !custom.is_empty() {
        return Some(json!(custom));
    }
    let sel: Vec<String> = draft
        .options
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if get(q, "multiSelect").as_bool().unwrap_or(false) {
        (!sel.is_empty()).then(|| json!(sel))
    } else {
        sel.first().map(|s| json!(s))
    }
}

/// Complete answers record for `thread.user-input.respond`
/// (upstream buildPendingUserInputAnswers): every question answered.
fn pi_build_answers(
    questions: &[Value],
    drafts: Option<&std::collections::HashMap<String, PiDraft>>,
) -> Option<Value> {
    let mut out = serde_json::Map::new();
    for q in questions {
        let qid = str_at(q, "id");
        let draft = drafts.and_then(|m| m.get(qid));
        out.insert(qid.to_string(), pi_resolved(q, draft)?);
    }
    Some(Value::Object(out))
}

/// Upstream withUserInputExchanges pairing: each `user-input.resolved`
/// activity joins the newest still-open `user-input.requested` sharing its
/// requestId + lifecycleGeneration. Returns resolved-id -> (questions,
/// answers) plus the consumed requested ids the transcript drops.
fn derive_ui_exchanges(
    activities: &[Value],
) -> (
    std::collections::HashMap<String, (Vec<Value>, Option<Value>)>,
    std::collections::HashSet<String>,
) {
    let mut open: Vec<(String, String, String, Vec<Value>)> = Vec::new();
    let mut pairs: std::collections::HashMap<String, (Vec<Value>, Option<Value>)> =
        std::collections::HashMap::new();
    let mut consumed = std::collections::HashSet::new();
    for a in activities {
        let kind = str_at(a, "kind");
        if kind != "user-input.requested" && kind != "user-input.resolved" {
            continue;
        }
        let payload = get(a, "payload");
        let rid = str_at(payload, "requestId").to_string();
        if rid.is_empty() {
            continue;
        }
        let gen = str_at(payload, "lifecycleGeneration").to_string();
        if kind == "user-input.requested" {
            // An invalid replacement must not leave an earlier question open.
            open.retain(|(r, g, _, _)| !(r == &rid && g == &gen));
            let qs: Vec<Value> = arr(payload, "questions")
                .iter()
                .filter(|q| {
                    !str_at(q, "id").is_empty()
                        && !str_at(q, "header").is_empty()
                        && !str_at(q, "question").is_empty()
                        && get(q, "options").is_array()
                })
                .cloned()
                .collect();
            if !qs.is_empty() {
                open.push((rid, gen, str_at(a, "id").to_string(), qs));
            }
        } else if let Some(pos) = open
            .iter()
            .rposition(|(r, g, _, _)| r == &rid && g == &gen)
        {
            let (_, _, req_id, qs) = open.remove(pos);
            consumed.insert(req_id);
            let answers = get(payload, "answers").clone();
            pairs.insert(
                str_at(a, "id").to_string(),
                (qs, (!answers.is_null()).then_some(answers)),
            );
        }
    }
    (pairs, consumed)
}

/// Upstream formatUserInputAnswer: answers[qid] or answers[question text],
/// string / string[] / {answers:[]}, joined ", ", None when empty.
fn ui_answer_text(answers: Option<&Value>, q: &Value) -> Option<String> {
    let answers = answers?;
    let value = answers
        .get(str_at(q, "id"))
        .or_else(|| answers.get(str_at(q, "question")))?;
    let parts: Vec<&str> = if let Some(s) = value.as_str() {
        vec![s]
    } else if let Some(list) = value.as_array() {
        list.iter().filter_map(|v| v.as_str()).collect()
    } else {
        arr(value, "answers")
            .iter()
            .filter_map(|v| v.as_str())
            .collect()
    };
    let text = parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    (!text.is_empty()).then_some(text)
}

/// Installed build version, mirrored from apps/web/package.json — WhatsNew
/// entries key on it exactly (upstream APP_VERSION).
const APP_VERSION: &str = "1.6.4";

/// The curated "What's new" dataset, converted verbatim from upstream
/// apps/web/src/whatsNew/entries.ts.
fn whats_new_entries() -> &'static [Value] {
    static E: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    E.get_or_init(|| {
        serde_json::from_str(include_str!("../whats_new.json")).unwrap_or_default()
    })
}

/// Upstream compareVersions: MAJOR.MINOR.PATCH tuple compare, non-numeric -> 0.
fn wn_cmp_versions(a: &str, b: &str) -> i64 {
    let parts = |v: &str| {
        let mut it = v.split('.');
        let n = |s: Option<&str>| s.and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
        (n(it.next()), n(it.next()), n(it.next()))
    };
    let (am, an, ap) = parts(a);
    let (bm, bn, bp) = parts(b);
    if am != bm {
        am - bm
    } else if an != bn {
        an - bn
    } else {
        ap - bp
    }
}

/// Entries sorted newest-first (upstream sortEntriesByVersionDesc).
fn whats_new_sorted() -> Vec<Value> {
    let mut e = whats_new_entries().to_vec();
    e.sort_by(|a, b| {
        wn_cmp_versions(str_at(b, "version"), str_at(a, "version")).cmp(&0)
    });
    e
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

// Upstream DEFAULT_PROVIDER_ORDER (PROVIDER_DESCRIPTORS order).
const PROVIDER_ORDER: [&str; 10] = [
    "codex", "claudeAgent", "cursor", "antigravity", "grok", "droid", "opencode", "pi", "devin", "omp",
];

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
    ("saved-logins", "Saved logins"),
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

/// serde number-or-numeric-string → f64, 0 on anything else.
fn num(v: Option<&Value>) -> f64 {
    v.and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
    .unwrap_or(0.0)
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

/// Collect remote image URLs from markdown `![alt](url)` references. The
/// transcript markdown renderer (gpui TextView) resolves document images
/// through the shell cache below instead of gpui's own resource loader,
/// which does not follow the redirects image hosts commonly answer with.
fn extract_md_image_urls(text: &str, out: &mut Vec<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'!' && bytes[i + 1] == b'[' {
            if let Some(close) = text[i + 2..].find("](") {
                let url_start = i + 2 + close + 2;
                if let Some(end) = text[url_start..].find(')') {
                    let url = text[url_start..url_start + end].trim();
                    if url.starts_with("http://") || url.starts_with("https://") {
                        let url = url.split_whitespace().next().unwrap_or(url);
                        out.push(url.to_string());
                    }
                    i = url_start + end;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Fetch and decode a remote image for the transcript cache. Runs on the
/// background executor; ureq follows redirects by default.
fn fetch_decode_image(url: &str) -> Option<std::sync::Arc<RenderImage>> {
    use std::io::Read as _;
    let mut req = ureq::get(url);
    if let (Some(base), Some(cookie)) = (
        crate::ws_client::BACKEND_BASE.get(),
        crate::ws_client::SESSION_COOKIE.get(),
    ) {
        if url.starts_with(base.as_str()) {
            req = req.set("Cookie", cookie);
        }
    }
    let resp = req.call().ok()?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(20 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?;
    let mut rgba = decoded.to_rgba8();
    // gpui frames are BGRA (same conversion as the platform image decoder).
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Some(std::sync::Arc::new(RenderImage::new(smallvec::smallvec![
        image::Frame::new(rgba)
    ])))
}

fn decode_bytes_to_render_image(bytes: &[u8]) -> Option<std::sync::Arc<RenderImage>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let mut rgba = decoded.to_rgba8();
    // gpui frames are BGRA (same conversion as the platform image decoder).
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Some(std::sync::Arc::new(RenderImage::new(smallvec::smallvec![
        image::Frame::new(rgba)
    ])))
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k).as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// Automation target projects — upstream excludes chat-kind implicit
/// containers; only real projects are selectable automation targets.
fn autom_projects(v: &Value) -> Vec<Value> {
    arr(v, "projects")
        .iter()
        .filter(|p| str_at(p, "kind") != "chat")
        .cloned()
        .collect()
}

// ── AsciiWaveBackground (upstream ambient field) ────────────────────────────
// Port of apps/web/src/components/chat/AsciiWaveBackground.tsx: three cycling
// noise patterns, per-cell glyph scramble, and accent click ripples.

const ASCII_GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
// Cell pitch in px, matching upstream FONT_SIZE_PX/LINE_HEIGHT_PX metrics.
// Upstream cellW = measureText("M").width + 1 in the UI font (~8.5px for
// SF Pro at 10px); narrower cells packed the field too densely.
const ASCII_CELL_W: f32 = 8.5;
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

/// Upstream `ui/switch.tsx` (sm): 32x20 track with a 1px border and 1px
/// padding, a 16px white thumb (shadow-sm + ring-black/5) travelling 12px.
/// Off = color-mix(foreground 20%, control-opaque) under a foreground/14%
/// border; control-opaque is the surface in light and rgb(23,23,23) on the
/// default dark #111111 surface (6/235 toward the ink).
/// Upstream `--color-text-accent`, measured on the live web app: switch
/// tracks and accent controls read this token, not `primary` (which is the
/// ink color at zero contrast). Dark #F58056 / light #C74614.
fn text_accent(dark: bool) -> gpui::Hsla {
    if dark {
        hsla(0.048, 0.89, 0.649, 1.0) // #F58056
    } else {
        hsla(0.048, 0.9, 0.45, 1.0) // #C74614 family
    }
}

fn switch_track(on: bool, accent: gpui::Hsla, theme: &gpui_kit::component::Theme) -> gpui::Div {
    let mix = |a: gpui::Hsla, b: gpui::Hsla, t: f32| -> gpui::Hsla {
        let (a, b) = (gpui::Rgba::from(a), gpui::Rgba::from(b));
        gpui::Rgba {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: 1.,
        }
        .into()
    };
    let dark = matches!(theme.mode, gpui_kit::component::ThemeMode::Dark);
    let control = if dark {
        mix(theme.background, theme.foreground, 6. / 235.)
    } else {
        theme.background
    };
    let (fill, border) = if on {
        (accent, accent)
    } else {
        (mix(control, theme.foreground, 0.2), theme.foreground.opacity(0.14))
    };
    let t = div()
        .w(px(32.))
        .h(px(20.))
        .flex_shrink_0()
        .rounded_full()
        .border_1()
        .border_color(border)
        .bg(fill)
        .p(px(1.))
        .flex()
        .items_center();
    let t = if on { t.justify_end() } else { t.justify_start() };
    t.child(
        div()
            .size(px(16.))
            .rounded_full()
            .bg(gpui::white())
            .shadow(vec![
                gpui::BoxShadow::new(px(0.), px(0.), gpui::hsla(0., 0., 0., 0.05))
                    .spread_radius(px(1.)),
                gpui::BoxShadow::new(px(0.), px(1.), gpui::hsla(0., 0., 0., 0.1))
                    .blur_radius(px(3.)),
                gpui::BoxShadow::new(px(0.), px(1.), gpui::hsla(0., 0., 0., 0.1))
                    .blur_radius(px(2.))
                    .spread_radius(px(-1.)),
            ]),
    )
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

/// Upstream describePullRequestState (pullRequestDetail.logic).
fn pr_state_label(state: &str, is_draft: bool) -> &'static str {
    if is_draft && state == "open" {
        "Draft"
    } else if state == "open" {
        "Ready for review"
    } else if state == "merged" {
        "Merged"
    } else {
        "Closed"
    }
}

/// Upstream githubActorName: display name, else login.
fn gh_actor_name(actor: &Value) -> String {
    let name = str_at(actor, "name");
    if !name.is_empty() {
        return name.to_string();
    }
    str_at(actor, "login").to_string()
}

/// Upstream summarizePullRequestChecks (environmentPullRequest.logic):
/// (label, tone) where tone is failure/pending/success/none.
fn pr_checks_summary(checks: &[Value]) -> (String, &'static str) {
    let count = |s: &str| {
        checks
            .iter()
            .filter(|c| str_at(c, "status") == s)
            .count()
    };
    let failing = count("failure");
    if failing > 0 {
        return (
            plural(failing as f64, "failing check", "failing checks"),
            "failure",
        );
    }
    let cancelled = count("cancelled");
    if cancelled > 0 {
        return (
            plural(cancelled as f64, "cancelled check", "cancelled checks"),
            "failure",
        );
    }
    let pending = count("pending");
    if pending > 0 {
        return (
            plural(pending as f64, "pending check", "pending checks"),
            "pending",
        );
    }
    if checks.is_empty() {
        return ("No checks".to_string(), "none");
    }
    if count("success") == 0 {
        return ("No required checks".to_string(), "none");
    }
    ("All checks passed".to_string(), "success")
}

fn pr_check_status_label(status: &str) -> &'static str {
    match status {
        "pending" => "Running",
        "success" => "Succeeded",
        "failure" => "Failed",
        "skipped" => "Skipped",
        "neutral" => "Neutral",
        "cancelled" => "Cancelled",
        _ => "Running",
    }
}

/// Upstream buildPullRequestTimelineEvents: creation, commits, comments,
/// terminal merge/close, chronologically sorted. (id, at, title, body)
fn pr_timeline_events(detail: &Value) -> Vec<(String, String, Option<String>)> {
    let mut events: Vec<(String, String, String, Option<String>)> = Vec::new();
    let author = gh_actor_name(get(detail, "author"));
    events.push((
        "created".to_string(),
        str_at(detail, "createdAt").to_string(),
        format!(
            "{} opened this pull request",
            if author.is_empty() { "Someone" } else { &author }
        ),
        None,
    ));
    for commit in arr(detail, "commits") {
        let oid = str_at(commit, "oid").to_string();
        let short: String = oid.chars().take(7).collect();
        let author_label = arr(commit, "authors")
            .iter()
            .map(gh_actor_name)
            .find(|n| !n.is_empty());
        let headline = str_at(commit, "messageHeadline").to_string();
        events.push((
            oid,
            str_at(commit, "committedDate").to_string(),
            match author_label {
                Some(a) => format!("Commit {short} by {a}"),
                None => format!("Commit {short}"),
            },
            Some(if headline.is_empty() {
                "No commit message.".to_string()
            } else {
                headline
            }),
        ));
    }
    for comment in arr(detail, "comments") {
        let who = gh_actor_name(get(comment, "author"));
        let verb = if str_at(comment, "kind") == "review" {
            "reviewed"
        } else {
            "commented"
        };
        let body = str_at(comment, "body").to_string();
        events.push((
            str_at(comment, "id").to_string(),
            str_at(comment, "createdAt").to_string(),
            format!(
                "{} {verb}",
                if who.is_empty() { "Someone" } else { &who }
            ),
            if body.is_empty() { None } else { Some(body) },
        ));
    }
    let merged_at = get(detail, "mergedAt").as_str().map(str::to_string);
    if let Some(m) = merged_at {
        events.push(("merged".to_string(), m, "Pull request merged".to_string(), None));
    } else if let Some(c) = get(detail, "closedAt").as_str().map(str::to_string) {
        events.push(("closed".to_string(), c, "Pull request closed".to_string(), None));
    }
    events.sort_by(|a, b| a.1.cmp(&b.1));
    events
        .into_iter()
        .map(|(_id, at, title, body)| (at, title, body))
        .collect()
}

/// Upstream summarizePullRequestComments: "No comments" / "N comment(s)" /
/// "N+ comments" when truncated.
fn pr_comments_summary(count: usize, truncated: bool) -> String {
    if count == 0 {
        return if truncated {
            "Comments may exist".to_string()
        } else {
            "No comments".to_string()
        };
    }
    let noun = if count == 1 { "comment" } else { "comments" };
    if truncated {
        format!("{count}+ {noun}")
    } else {
        format!("{count} {noun}")
    }
}

/// Upstream pullRequestStackTargetEntries + assessPullRequestStack.
/// Returns (label, tone, mergeTargetCount, canAttemptMerge, blocker).
fn pr_stack_assessment(
    stack: &Value,
) -> (&'static str, &'static str, usize, bool, Option<String>) {
    let position = get(stack, "position").as_i64().unwrap_or(0);
    let targets: Vec<&Value> = arr(stack, "entries")
        .iter()
        .filter(|e| {
            get(e, "position").as_i64().unwrap_or(0) <= position
                && str_at(e, "state") != "merged"
        })
        .collect();
    if targets.is_empty() {
        return ("Stack merged", "complete", 0, false, None);
    }
    let n = targets.len();
    if let Some(closed) = targets.iter().find(|e| str_at(e, "state") == "closed") {
        return (
            "Stack needs attention",
            "blocked",
            n,
            false,
            Some(format!(
                "#{} is closed without being merged.",
                get(closed, "number").as_i64().unwrap_or(0)
            )),
        );
    }
    if let Some(draft) = targets
        .iter()
        .find(|e| get(e, "isDraft").as_bool().unwrap_or(false))
    {
        return (
            "Stack needs attention",
            "blocked",
            n,
            false,
            Some(format!(
                "#{} is still a draft.",
                get(draft, "number").as_i64().unwrap_or(0)
            )),
        );
    }
    if let Some(conflicting) = targets.iter().find(|e| {
        str_at(e, "mergeability") == "conflicting"
            || matches!(
                get(e, "mergeStateStatus").as_str().unwrap_or(""),
                "BLOCKED" | "DIRTY" | "DRAFT"
            )
    }) {
        return (
            "Stack needs attention",
            "blocked",
            n,
            false,
            Some(format!(
                "#{} is not ready to merge.",
                get(conflicting, "number").as_i64().unwrap_or(0)
            )),
        );
    }
    if targets
        .iter()
        .any(|e| str_at(e, "mergeStateStatus") == "UNSTABLE")
    {
        return ("Mergeable with failing checks", "warning", n, true, None);
    }
    let pending = targets.iter().any(|e| {
        str_at(e, "mergeability") == "unknown"
            || get(e, "mergeStateStatus").as_str().is_none()
            || matches!(str_at(e, "mergeStateStatus"), "BEHIND" | "UNKNOWN")
    });
    if pending {
        return ("Merge status pending", "pending", n, true, None);
    }
    ("Ready to merge", "ready", n, true, None)
}

/// Upstream pullRequestMergeBlocker.
fn pr_merge_blocker(detail: &Value) -> Option<String> {
    if get(detail, "stackMetadataIncomplete").as_bool() == Some(true) {
        return Some(
            "Stack details are temporarily unavailable. Refresh before merging."
                .to_string(),
        );
    }
    let stack = get(detail, "stack");
    if !stack.is_null() {
        let (_, _, _, can_attempt, blocker) = pr_stack_assessment(stack);
        if !can_attempt {
            return Some(
                blocker.unwrap_or_else(|| "This stack is not ready to merge.".to_string()),
            );
        }
    }
    if str_at(detail, "mergeability") == "conflicting" {
        return Some("Resolve merge conflicts before merging".to_string());
    }
    None
}

/// Upstream resolvePullRequestPrimaryAction: "ready" for drafts, "merge"
/// (possibly blocked) for open PRs, None when merged/closed or nothing allowed.
/// Returns ("ready" | "merge", blockedReason, stackCount).
fn pr_primary_action(detail: &Value) -> Option<(&'static str, Option<String>, Option<usize>)> {
    if str_at(detail, "state") != "open" {
        return None;
    }
    if get(detail, "isDraft").as_bool().unwrap_or(false) {
        return Some(("ready", None, None));
    }
    let allowed = ["merge", "squash", "rebase"]
        .iter()
        .filter(|m| {
            get(get(detail, "mergeCapabilities"), m)
                .as_bool()
                .unwrap_or(false)
        })
        .count();
    let blocker = pr_merge_blocker(detail);
    if blocker.is_none() && allowed == 0 {
        return None;
    }
    let stack_count = if get(detail, "stack").is_null() {
        None
    } else {
        Some(pr_stack_assessment(get(detail, "stack")).2)
    };
    Some(("merge", blocker, stack_count))
}

/// Upstream formatClockElapsed + formatClockDuration: "20s", "3m", "1h 2m",
/// "2d 5h" between two ISO stamps; None when unparseable or inverted.
fn clock_elapsed(start_iso: &str, end_iso: &str) -> Option<String> {
    let s = chrono::DateTime::parse_from_rfc3339(start_iso).ok()?;
    let e = chrono::DateTime::parse_from_rfc3339(end_iso).ok()?;
    let secs = e.signed_duration_since(s).num_seconds();
    if secs < 0 {
        return None;
    }
    if secs < 60 {
        return Some(format!("{secs}s"));
    }
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    if days > 0 {
        return Some(if hours > 0 {
            format!("{days}d {hours}h")
        } else {
            format!("{days}d")
        });
    }
    let minutes = (secs % 3_600) / 60;
    let seconds = secs % 60;
    if hours > 0 {
        Some(if minutes > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{hours}h")
        })
    } else {
        Some(if seconds > 0 {
            format!("{minutes}m {seconds}s")
        } else {
            format!("{minutes}m")
        })
    }
}

// Local wall-clock time "h:mm AM/PM" like the upstream message footer.
fn clock_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    t.with_timezone(&chrono::Local).format("%-I:%M %p").to_string()
}

// Epoch-ms → RFC3339 (upstream toISOString, accepted by stats.getRecap).
fn ms_to_iso(ms: f64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(now_iso)
}

/// Upstream resolveInboxDay (`packages/shared/src/inboxDay.ts`): the working
/// day containing `now_ms` runs 4:00→4:00 local, split at 12:00 and 18:00
/// (Morning / Afternoon / Evening). Returns (fromMs, toMs, slotBoundaries).
fn inbox_day_range_at(now_ms: f64) -> (f64, f64, Vec<f64>) {
    let local = chrono::Local::now();
    let mut day = local
        .date_naive()
        .and_hms_opt(4, 0, 0)
        .and_then(|d| d.and_local_timezone(chrono::Local).earliest())
        .unwrap_or(local);
    // Walk back until the 4:00 window covers `now_ms` (yesterday's range).
    while day.timestamp_millis() as f64 > now_ms {
        day = day - chrono::Duration::days(1);
    }
    let noon = day + chrono::Duration::hours(8);
    let evening = day + chrono::Duration::hours(14);
    let end = day + chrono::Duration::days(1);
    (
        day.timestamp_millis() as f64,
        end.timestamp_millis() as f64,
        vec![
            noon.timestamp_millis() as f64,
            evening.timestamp_millis() as f64,
        ],
    )
}

fn inbox_day_range() -> (f64, f64, Vec<f64>) {
    inbox_day_range_at(chrono::Local::now().timestamp_millis() as f64)
}

/// Upstream plural(): "1 chat" / "2 chats".
fn plural(n: f64, one: &str, many: &str) -> String {
    if n == 1.0 {
        format!("1 {one}")
    } else {
        format!("{} {many}", n as i64)
    }
}

/// Upstream formatCompact for token counts: 990 → "990", 8_400 → "8.4K",
/// 3_550_000 → "3.6M".
fn fmt_compact(n: f64) -> String {
    if n < 1_000.0 {
        format!("{}", n as i64)
    } else if n < 1_000_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        format!("{:.1}M", n / 1_000_000.0)
    }
}

/// recap.tokens = {user, automation, agent} → sum.
fn recap_tokens(v: &Value) -> f64 {
    let t = v.get("tokens");
    num(t.and_then(|t| t.get("user")))
        + num(t.and_then(|t| t.get("automation")))
        + num(t.and_then(|t| t.get("agent")))
}

/// "13h 48m" / "3m" / "45s" from milliseconds (upstream formatDuration).
fn fmt_dur_ms(ms: f64) -> String {
    let s = (ms / 1000.0) as i64;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else {
        let h = s / 3600;
        let m = (s % 3600) / 60;
        if m > 0 { format!("{h}h {m}m") } else { format!("{h}h") }
    }
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

fn desktop_update_state_from_status(status: &Value) -> Option<crate::updater::UpdateState> {
    serde_json::from_value(status.get("state").cloned()?).ok()
}

fn desktop_update_state_from_result(result: &Value) -> Option<crate::updater::UpdateState> {
    result.get("status").and_then(desktop_update_state_from_status)
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
                                // voiceEnterBehavior: while a voice note is
                                // recording Enter stops it — "stop" transcribes
                                // into the composer, "send" sends the message
                                // once the transcription lands.
                                if this.voice_capture.is_some() {
                                    let behavior =
                                        this.setting_str("voiceEnterBehavior", "stop");
                                    this.voice_send_after = behavior == "send";
                                    this.stop_voice(window, cx);
                                } else {
                                    this.send(window, cx);
                                }
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
                            if sid == "saved-logins" {
                                this.refresh_vault(cx);
                                this.refresh_cookie_sources(cx);
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
            let gh_search = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search or paste a PR link")
            });
            cx.subscribe_in(
                &gh_search,
                window,
                |this: &mut Self, _state, event, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.notify();
                    }
                },
            )
            .detach();
            cx.bind_keys([
                KeyBinding::new("cmd-f", ToggleFind, Some("native")),
                KeyBinding::new("cmd-j", ToggleTerminal, Some("native")),
            ]);
            // Theme pack editor sliders (upstream ThemePackEditor): one
            // SliderState per variant x {opacity, blur, contrast}; slider
            // writes land in themePacks[variant] via tp_slider_change.
            let mut tp_sliders: std::collections::HashMap<String, Entity<SliderState>> =
                std::collections::HashMap::new();
            for (variant, key, min, max, init) in [
                ("dark", "opacity", 15., 100., 90.),
                ("dark", "blur", 1., 64., 64.),
                ("dark", "contrast", 0., 100., 0.),
                ("light", "opacity", 15., 100., 90.),
                ("light", "blur", 1., 64., 64.),
                ("light", "contrast", 0., 100., 0.),
            ] {
                let st = cx.new(|_| {
                    SliderState::new()
                        .min(min)
                        .max(max)
                        .step(1.)
                        .default_value(init)
                });
                let id = format!("{variant}-{key}");
                let id2 = id.clone();
                cx.subscribe(&st, move |this, _st, ev: &SliderEvent, cx| {
                    if let SliderEvent::Change(v) = ev {
                        this.tp_slider_change(&id2, v.start(), cx);
                    }
                })
                .detach();
                tp_sliders.insert(id, st);
            }
            let vault_master_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Master password")
            });
            vault_master_input.update(cx, |state, cx| state.set_masked(true, window, cx));
            let vault_confirm_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Confirm master password")
            });
            vault_confirm_input.update(cx, |state, cx| state.set_masked(true, window, cx));
            let vault_reveal_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Master password")
            });
            vault_reveal_input.update(cx, |state, cx| state.set_masked(true, window, cx));
            cx.subscribe_in(
                &vault_master_input,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        let configured = this
                            .vault_snapshot
                            .as_ref()
                            .and_then(|snapshot| snapshot.pointer("/protection/configured"))
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        this.submit_vault_master(!configured, window, cx);
                    }
                },
            )
            .detach();
            cx.subscribe_in(
                &vault_reveal_input,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        this.submit_vault_reveal(window, cx);
                    }
                },
            )
            .detach();
            let cookie_domain_input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("example.com (optional)")
            });
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
                preview_pages: std::collections::HashMap::new(),
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
                inbox_recap_prev: None,
                inbox_usage: Vec::new(),
                inbox_signed_in: None,
                inbox_updates: Vec::new(),
                tools_expanded: false,
                subagents_expanded: false,
                pending_attachments: Vec::new(),
                attachment_thumbs: std::collections::HashMap::new(),
                interaction_mode: "default",
                fast_mode: false,
                composer_model: (
                    "codex".to_string(),
                    "gpt-6-astra".to_string(),
                    "GPT-6 Astra".to_string(),
                ),
                model_picker_open: false,
                model_picker_search: cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Search models...")
                }),
                model_catalog: Vec::new(),
                composer_effort_idx: 2,
                model_picker_provider: "codex".to_string(),
                queued_turns: Vec::new(),
                queue_paused: false,
                task_card_open: false,
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
    backdrop_full: false,
            ascii_t0: std::time::Instant::now(),
            ascii_seed: 0.0,
            ascii_cells: Vec::new(),
            backdrop_img: None,
            backdrop_field: None,
            backdrop_spec: None,
            backdrop_key: (0, 0, false),
            atmosphere_img: None,
            atmosphere_key: (0, 0, false, 0),
            ascii_ripples: Vec::new(),
            ascii_cell_w: 0.0,
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
                changes_diff_state: None,
                gh_items: Vec::new(),
                gh_errors: Vec::new(),
                gh_viewer: String::new(),
                gh_loading: false,
                gh_epoch: 0,
                gh_kind: "all",
                gh_state_filter: "open",
                gh_sort: "created",
                gh_search,
                pr_sel: None,
                issue_sel: None,
                issue_detail: None,
                issue_detail_loading: false,
                issue_detail_tab: 0,
                issue_detail_epoch: 0,
                issue_comment_input: None,
                issue_comment_posting: false,
                pr_detail: None,
                pr_detail_loading: false,
                pr_detail_tab: 0,
                pr_detail_epoch: 0,
                pr_diff_state: None,
                pr_diff_loading: false,
                pr_diff_error: None,
                pr_diff_truncated: false,
                pr_disc: [true, true, true],
                pr_comment_input: None,
                pr_actions_menu: None,
                pr_stack_menu: None,
                pr_confirm: None,
                pr_merge_method: "merge".to_string(),
                pr_comments_closed: std::collections::HashSet::new(),
                pr_diff_files: Vec::new(),
                pr_summary_scroll: ScrollHandle::new(),
                issue_scroll: ScrollHandle::new(),
                pr_timeline_scroll: ScrollHandle::new(),
                pr_comment_posting: false,
                blame_pop: None,
                changes_loading: false,
                changes_unavailable: false,
                dock_tab: 0,
                view_terminal: false,
                view_browser: false,
                browser_webview: None,
                browser_input: None,
                view_cua: false,
                cua: None,
                cua_seen_seq: 0,
                cua_screen_points: None,
                cua_input: None,
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
                vault_snapshot: None,
                vault_error: None,
                vault_master_input,
                vault_confirm_input,
                vault_reveal_input,
                vault_reveal_id: None,
                vault_revealed: None,
                vault_delete_id: None,
                vault_prompts_seen: std::collections::HashSet::new(),
                cookie_sources: Vec::new(),
                cookie_browser: None,
                cookie_profile: None,
                cookie_domain_input,
                cookie_importing: false,
                cookie_profile_confirmed: false,
                cookie_result: None,
                cookie_error: None,
                dismissed_status: std::collections::HashMap::new(),
                visited_threads: std::collections::HashMap::new(),
                keep_awake_mode: 2,
                notification_primed: false,
                notification_states: std::collections::HashMap::new(),
                notification_keys: std::collections::HashSet::new(),
                notification_window: None,
                notification_window_active: true,
                desktop_update_state: None,
                desktop_update_installing: false,
                beta_action_running: None,
                beta_action_message: None,
                coachmarks_seen: std::collections::HashSet::new(),
                feature_flags: [false; 3],
                flags_menu_open: false,
                project_picker_open: false,
                landing_project_id: None,
            landing_temporary: false,
                project_picker_input: None,
                dismissed_thread_errors: std::collections::HashSet::new(),
                error_details_open: false,
                error_details_for: None,
                unblocking_thread: false,
                provider_statuses: Vec::new(),
                dismissed_provider_banner_keys: std::collections::HashSet::new(),
                plan_expanded: std::collections::HashSet::new(),
                starting_sidechat: false,
                pr_dialog_open: false,
                pr_dialog_input: None,
                pr_resolved: None,
                pr_resolving: false,
                pr_dialog_error: None,
                pr_busy: false,
                pr_resolve_epoch: 0,
                coachmark: None,
                sponsor_dismissed: false,
            search_open: false,
            sidebar_collapsed: false,
            user_input_open: Vec::new(),
            md_images: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            md_images_pending: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            activity_open: Vec::new(),
            turn_files_collapsed: Vec::new(),
            voice_capture: None,
            voice_transcribing: false,
            voice_send_after: false,
            ctx_menu: None,
            ctx_menu_tab: false,
            ctx_sub: None,
            file_ctx_menu: None,
            hover_thread: None,
            hover_card: None,
            hover_card_seq: 0,
            ctx_confirm_delete: None,
            proj_ctx_menu: None,
            proj_ctx_confirm: false,
            hover_project: None,
            hover_user_msg: None,
            project_card: None,
            project_card_seq: 0,
            proj_rename: None,
            proj_rename_input: None,
            proj_relocate: None,
            proj_relocate_input: None,
            import_dlg: None,
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
            snooze_dlg_tid: None,
            snooze_dlg_input: None,
            autom_defs: Vec::new(),
            autom_runs: Vec::new(),
            autom_sel: None,
            autom_dialog: None,
            wn_popout: false,
            wn_view: 0,
            wn_changelog_open: std::collections::HashSet::new(),
            wn_seen: None,
            theme_packs: json!({}),
            tp_edit: None,
            tp_input: None,
            tp_import: None,
            tp_sliders,
            git_dialog: None,
            git_msg_input: None,
            git_pr_title_input: None,
            git_pr_body_input: None,
            git_pr_draft: false,
            git_commit_action: "commit".to_string(),
            git_action_running: false,
            tour_step: None,
            user_input_draft: Vec::new(),
            pi_drafts: std::collections::HashMap::new(),
            pi_qidx: std::collections::HashMap::new(),
            pi_responding: false,
            find_open: false,
                find_input: None,
                find_query: String::new(),
            };
            this.refresh(cx);
            this.fetch_settings(cx);
            // Keep the component theme in step with the system appearance, not
            // only when settings load: the chrome follows it (`dark_ui`), so a
            // late or changed appearance left theme text dark-on-dark.
            cx.observe_window_appearance(window, |this: &mut Self, window, cx| {
                this.apply_appearance_mode(Some(window), cx);
                this.apply_theme_pack(cx);
            })
            .detach();
            // Give the root view initial focus so global chords dispatch even
            // before an input takes focus (chords bubble from inputs anyway).
            this.focus_handle.focus(window, cx);
            // Owner vault prompts originate in the CDP browser host, outside the
            // server websocket. Poll only while the owner surface is visible.
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                loop {
                    let fast = this
                        .update(cx, |this: &mut Self, _cx| {
                            this.view_settings && this.settings_section == "saved-logins"
                        })
                        .unwrap_or(false);
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(if fast { 500 } else { 3000 }))
                        .await;
                    let keep = this
                        .update(cx, |this: &mut Self, cx| {
                            if let Some(Ok(vault)) = browserhost_rs::cdp::initialized_browser_vault() {
                                if let Ok(snapshot) = vault.snapshot() {
                                    let pending: Vec<String> = arr(&snapshot, "pending")
                                        .iter()
                                        .map(|prompt| str_at(prompt, "id").to_string())
                                        .filter(|id| !id.is_empty() && !this.vault_prompts_seen.contains(id))
                                        .collect();
                                    if !pending.is_empty() {
                                        for id in pending {
                                            this.vault_prompts_seen.insert(id);
                                        }
                                        if !(this.view_settings && this.settings_section == "saved-logins") {
                                            this.clear_views();
                                            this.view_settings = true;
                                            this.settings_section = "saved-logins";
                                        }
                                    }
                                }
                            }
                            if this.view_settings && this.settings_section == "saved-logins" {
                                this.refresh_vault(cx);
                            }
                        })
                        .is_ok();
                    if !keep {
                        break;
                    }
                }
            })
            .detach();
            // Desktop updater state is owned by the shell service and changes
            // outside websocket events (background checks, transfers, restore).
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(5000))
                        .await;
                    let keep = this
                        .update(cx, |this: &mut Self, cx| this.refresh_desktop_update_state(cx))
                        .is_ok();
                    if !keep { break; }
                }
            })
            .detach();
            // AsciiWaveBackground ambient clock: ~15fps repaints while the
            // landing field is on screen (upstream AMBIENT_FRAME_INTERVAL_MS).
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                loop {
                    // Upstream cadence: 15fps ambient while ripples run, 5fps
                    // once the field is idle (IDLE_FRAME_INTERVAL_MS).
                    let ripples = this
                        .update(cx, |this: &mut Self, _cx| {
                            this.show_ascii_bg
                                && this.landing_active()
                                && !this.ascii_ripples.is_empty()
                        })
                        .unwrap_or(false);
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(
                            if ripples { 66 } else { 1000 },
                        ))
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






































































    /// Global keymap: the upstream DEFAULT_SHORTCUT_FALLBACKS chords whose
    /// targets exist on the native surface. cmd-f / cmd-j stay declared
    /// KeyBindings (chat.find / terminal.toggle); the rest is dispatched here.
    // ---- shortcut recorder + keybinding dispatch ------------------------










































    /// Upstream FEEDBACK_CATEGORIES: (value, label, summary lead).
    const FEEDBACK_CATEGORIES: [(&str, &str, &str); 6] = [
        ("bug", "Bug", "I ran into a bug"),
        ("session", "Session", "I hit a session problem"),
        ("ui", "UI", "Something looked wrong"),
        ("performance", "Performance", "Synara felt slow"),
        ("idea", "Idea", "I have an idea"),
        ("other", "Other", "I have some feedback"),
    ];
















    // ── Automations ─────────────────────────────────────────────────────





















































































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
        self.notification_window = Some(window.window_handle());
        self.notification_window_active = window.is_window_active();
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
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let popover_bg: gpui::Hsla = if dark_ui {
            rgb(0x161616).into()
        } else {
            theme.popover
        };

        let ws_parts = self.build_workspace(window, cx);
        let workspace::NativeWorkspace {
            sidebar,
            pane,
            inbox_pane,
            dev_pane,
            files_pane,
            mut update_toast,
            composer_bar,
            split_bar,
            split_pane,
            provider_banner,
            thread_error_banner,
            query,
            mut toast_shown,
            has_thread,
            show_landing,
            thread_v
        } = ws_parts;


        let pane_parts = self.build_panes(window, cx);
        let panes::NativePanes {
            settings_pane,
            tasks_pane,
            kanban_pane,
            pullreq_pane,
            autom_pane,
            studio_pane,
            term_surface,
            settings_nav_el,
            pullreq_panel,
            autom_panel,
            studio_panel,
            kanban_header,
            kanban_count
        } = pane_parts;

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
        // The browser panel only lives in the thread workspace; when any
        // other surface owns the main column the native overlay must close
        // or it floats over content it does not belong to.
        let other_surface_active = self.view_files
            || self.view_dev
            || self.view_inbox
            || self.view_settings
            || self.view_tasks
            || self.view_kanban
            || self.view_pullreq
            || self.view_autom
            || self.view_studio
            || self.view_spaces;
        if self.view_browser && other_surface_active {
            self.view_browser = false;
            self.browser_webview = None;
            self.browser_input = None;
        }
        // The content surface is always opaque (Kartik's shell-atmosphere
        // model): in full-window mode it is what hides the persistent texture
        // behind an active thread — the atmosphere reads only through the
        // transparent chrome. The landing paints its own texture instance on
        // top of this surface, so empty chats are unaffected.
        // Upstream content card: inset 3px (--app-rail-inset-gap) from the
        // right and bottom window edges, bottom corners rounded 13px — the
        // shell atmosphere frame peeks through the gap.
        let mut main = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .relative()
            .mr(px(3.))
            .mb(px(3.))
            .rounded_br(px(13.))
            .rounded_bl(px(13.))
            .overflow_hidden();
        if !show_landing || other_surface_active {
            // The content surface is opaque over active threads and the
            // utility surfaces (tasks/inbox/settings/...): it is what covers
            // the persistent shell atmosphere, so the texture reads only
            // through the chrome strips. The empty landing stays clear —
            // its own texture instance shows through.
            main = main.bg(content_bg);
        }
                // Full-window atmosphere (upstream .app-shell-atmosphere):
                // window-coordinated texture, composited at 35% (light) / 80%
                // (dark) under the landing texture — the fork's two-canvas
                // structure. Never overwrites backdrop_img: the landing canvas
                // keeps its own pane-sized geometry.
                if self.show_ascii_bg {
                    let vpw = window.viewport_size().width.as_f32();
                    let vph = window.viewport_size().height.as_f32();
                    let dark = theme.is_dark();
                    let key = (vpw as u32, vph as u32, dark, self.backdrop_full as u32);
                    if self.atmosphere_key != key || self.atmosphere_img.is_none() {
                        // Upstream .app-shell-atmosphere weight: 35% light in
                        // both window modes; 80% dark over the chrome, but in
                        // full-window mode it settles to 60% AND dims
                        // (brightness 0.75 saturate 0.6 — #f2612d becomes
                        // rgb(146,81,57)) so the spread texture does not
                        // drown the content.
                        let (accent, alpha_scale) = if dark {
                            if self.backdrop_full {
                                ([146, 81, 57], 0.6)
                            } else {
                                ([0xf2, 0x61, 0x2d], 0.8)
                            }
                        } else {
                            ([0xc7, 0x46, 0x14], 0.35)
                        };
                        self.atmosphere_img = crate::backdrop::build_backdrop_image(
                            &crate::backdrop::BackdropSpec {
                                width: vpw,
                                height: vph,
                                dark,
                                accent_rgb: accent,
                                quiet: Vec::new(),
                                scale: window.scale_factor(),
                                alpha_scale,
                                // Upstream .app-shell-atmosphere clip-path:
                                // in normal window mode the texture prints
                                // only in the top strip (44px), the rail
                                // (51px) and the 3px outer gaps. Full-window
                                // mode lifts the clip.
                                chrome_clip: if self.backdrop_full {
                                    None
                                } else {
                                    Some((51.0, 44.0, 3.0))
                                },
                            },
                        );
                        self.atmosphere_key = key;
                    }
                }
                let atmosphere_img_c = if self.show_ascii_bg {
                    self.atmosphere_img.clone()
                } else {
                    None
                };
                let mut atmosphere_layer = if self.show_ascii_bg {
                    self.atmosphere_img.clone().map(|img| {
                        let vpw = window.viewport_size().width.as_f32();
                        let vph = window.viewport_size().height.as_f32();
                        gpui::img(ImageSource::Render(img))
                            .absolute()
                            .top_0()
                            .left_0()
                            .w(px(vpw))
                            .h(px(vph))
                    })
                } else {
                    None
                };
                let mut landing_bd_main = if show_landing && !other_surface_active && self.show_ascii_bg {
                    // Live ripple frames: while any click ripple is alive the
                    // bitmap is re-rasterized per frame from the cached density
                    // field (upstream re-dithers ring cells at 25fps).
                    let now = std::time::Instant::now();
                    let live: Vec<(f32, f32, f32, f32, f32)> = self
                        .ascii_ripples
                        .iter()
                        .map(|r| {
                            (
                                r.0,
                                r.1,
                                r.2,
                                r.3,
                                now.saturating_duration_since(r.4).as_millis() as f32,
                            )
                        })
                        .filter(|r| r.4 < crate::backdrop::RIPPLE_LIFETIME_MS)
                        .collect();
                    let frame_img = if !live.is_empty() {
                        match (&self.backdrop_spec, &self.backdrop_field) {
                            (Some(spec), Some(field)) => {
                                crate::backdrop::rasterize_backdrop(spec, field, &live)
                                    .or_else(|| self.backdrop_img.clone())
                            }
                            _ => self.backdrop_img.clone(),
                        }
                    } else {
                        self.backdrop_img.clone()
                    };
                    frame_img.map(|img| {
                        // Window-sized canvas mounted at the root: the scene
                        // continues under the floating sidebar (upstream
                        // data-sidebar-layout=rail empty-chat geometry).
                        let vpw = window.viewport_size().width.as_f32();
                        let vph = window.viewport_size().height.as_f32();
                        let mut field = div().size_full().overflow_hidden().relative();
                        // Window-coordinated atmosphere under the landing
                        // texture (upstream .app-shell-atmosphere at 35%/80%).
                        // In full-window mode the body-level layer already
                        // paints it at full strength — skip the dim copy.
                        // The body-level shell atmosphere (35%/80% frame
                        // weight) already reads through the transparent
                        // landing field — no separate dim copy needed.
                        if !theme.is_dark() {
                            field = field.child(
                                div()
                                    .absolute()
                                    .top(px(44.))
                                    .left(px(308.))
                                    .w(px((vpw - 308.0).max(200.0)))
                                    .h(px((vph - 44.0).max(200.0)))
                                    .bg(gpui::linear_gradient(
                                        180.,
                                        gpui::linear_color_stop(
                                            rgb(0xc74614).opacity(0.015),
                                            0.,
                                        ),
                                        gpui::linear_color_stop(
                                            rgb(0xc74614).opacity(0.045),
                                            0.55,
                                        ),
                                    )),
                            );
                        }
                        div()
                            .absolute()
                            .inset_0()
                            .overflow_hidden()
                            .child(field.child(
                                gpui::img(ImageSource::Render(img))
                                    .absolute()
                                    .top(px(44.))
                                    .left(px(308.))
                                    .w(px((vpw - 308.0).max(200.0)))
                                    .h(px((vph - 44.0).max(200.0))),
                            ))
                    })
                } else {
                    None
                };
        // landing_bd_main mounts at the window root below, behind strip+body,
        // so the texture runs under the transparent rail/sidebar chrome.
        let landing_bd_root = landing_bd_main.take();
        if self.view_settings {
            main = main.child(div().flex_1().child(settings_pane));
        } else if self.view_dev {
            main = main.child(div().flex_1().min_h_0().child(dev_pane));
        } else if self.view_files {
            main = main.child(div().flex_1().min_h_0().child(files_pane));
        } else if self.view_inbox {
            main = main.child(div().flex_1().min_h_0().child(inbox_pane));
        } else if self.view_tasks {
            main = main.child(div().flex_1().child(tasks_pane));
        } else if self.view_kanban {
            main = main
                .child(
                    // Header sits outside the scroll area so the top-bar
                    // overlay isn't clipped.
                    div()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .child(kanban_pane)
                        .child(kanban_header),
                );
        } else if self.view_pullreq {
            main = main.child(div().flex_1().child(pullreq_pane));
        } else if self.view_autom {
            main = main.child(div().flex_1().child(autom_pane));
        } else if self.view_studio {
            main = main.child(div().flex_1().child(studio_pane));
        } else {
            if self.view_browser {
                let mut browser_surface = v_flex().flex_1().min_h_0().w_full().bg(content_bg);
                let mut strip = h_flex()
                    .w_full()
                    .h(px(36.))
                    .px_2()
                    .items_center()
                    .gap_1()
                    .border_b_1()
                    .border_color(theme.border);
                if let Some(webview) = &self.browser_webview {
                    let back_wv = webview.clone();
                    let fwd_wv = webview.clone();
                    strip = strip
                        .child(
                            Button::new("browser-back")
                                .ghost()
                                .compact()
                                .icon(IconName::ChevronLeft)
                                .on_click(cx.listener(move |_this, _ev, _w, cx| {
                                    back_wv.update(cx, |view, _| {
                                        let _ = view.back();
                                    });
                                })),
                        )
                        .child(
                            Button::new("browser-forward")
                                .ghost()
                                .compact()
                                .icon(IconName::ChevronRight)
                                .on_click(cx.listener(move |_this, _ev, _w, cx| {
                                    fwd_wv.update(cx, |view, _| {
                                        let _ = view.forward();
                                    });
                                })),
                        );
                }
                if let Some(input) = &self.browser_input {
                    strip = strip.child(div().flex_1().child(Input::new(input)));
                }
                browser_surface = browser_surface.child(strip);
                if let Some(webview) = &self.browser_webview {
                    browser_surface = browser_surface.child(
                        div().flex_1().min_h_0().child(webview.clone()),
                    );
                }
                main = main.child(browser_surface);
            } else if self.view_cua {
                let (connected, error, seq) = self
                    .cua
                    .as_ref()
                    .and_then(|h| {
                        h.shared.lock().ok().map(|s| {
                            (s.connected, s.error.clone(), s.sequence)
                        })
                    })
                    .unwrap_or((false, None, 0));
                let thread_for_control = self
                    .selected_thread
                    .clone()
                    .unwrap_or_else(|| "shell".to_string());
                let mut cua_surface = v_flex().flex_1().min_h_0().w_full().bg(content_bg);
                // Control strip: enable / emergency stop, status text.
                let enable_thread = thread_for_control.clone();
                let stop_thread = thread_for_control.clone();
                let strip = h_flex()
                    .w_full()
                    .h(px(36.))
                    .px_3()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Computer"),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("cua-enable")
                            .compact()
                            .label("Enable control")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                let client_cell = this.client.clone();
                                let thread = enable_thread.clone();
                                cx.spawn(async move |_this, cx: &mut AsyncApp| {
                                    let cell = client_cell.clone();
                                    if let Some(client) = cx
                                        .background_executor()
                                        .spawn(async move { ensure_client(&cell) })
                                        .await
                                    {
                                        let thread2 = thread.clone();
                                        let _ = cx
                                            .background_executor()
                                            .spawn(async move {
                                                crate::cua::enable_control(&client, &thread2)
                                            })
                                            .await;
                                    }
                                })
                                .detach();
                            })),
                    )
                    .child(
                        Button::new("cua-stop")
                            .compact()
                            .label("Stop")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                let client_cell = this.client.clone();
                                let thread = stop_thread.clone();
                                cx.spawn(async move |_this, cx: &mut AsyncApp| {
                                    let cell = client_cell.clone();
                                    if let Some(client) = cx
                                        .background_executor()
                                        .spawn(async move { ensure_client(&cell) })
                                        .await
                                    {
                                        let thread2 = thread.clone();
                                        let _ = cx
                                            .background_executor()
                                            .spawn(async move {
                                                crate::cua::disable_control(&client, &thread2)
                                            })
                                            .await;
                                    }
                                })
                                .detach();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(if connected {
                                format!("live · frame {seq}")
                            } else if let Some(e) = &error {
                                format!("offline · {e}")
                            } else {
                                "connecting…".to_string()
                            }),
                    );
                cua_surface = cua_surface.child(strip);
                if let Some(input) = &self.cua_input {
                    cua_surface = cua_surface.child(
                        h_flex()
                            .w_full()
                            .h(px(34.))
                            .px_2()
                            .items_center()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(div().flex_1().child(Input::new(input))),
                    );
                }
                // The desktop still renders at logical screen size — retina
                // captures are 2× the point space — inside a scroll area, so
                // a click position in the element IS the screen point.
                if let Some(path) = self.cua_frame_path() {
                    let (pw, ph) = self.cua_png_size().unwrap_or((1440., 900.));
                    // Logical size comes from the live screen size when known;
                    // the retina 2x fallback only covers the first frames.
                    let (logical_w, logical_h) = self
                        .cua_screen_points
                        .map(|(w, h)| (w as f32, h as f32))
                        .unwrap_or((pw / 2.0, ph / 2.0));
                    let click_thread = thread_for_control.clone();
                    // Mouse positions arrive in window coordinates; a
                    // transparent canvas over the image records its painted
                    // origin every frame, so the click handler subtracts the
                    // true element origin — scroll offsets included.
                    let origin_slot = std::sync::Arc::new(std::sync::Mutex::new((0f32, 0f32)));
                    let origin_paint = origin_slot.clone();
                    let origin_click = origin_slot.clone();
                    let recorder = gpui::canvas(
                        move |_bounds, _window, _cx| (),
                        move |bounds, _unit, _window, _cx| {
                            if let Ok(mut slot) = origin_paint.lock() {
                                let x: f32 = bounds.origin.x.into();
                                let y: f32 = bounds.origin.y.into();
                                *slot = (x, y);
                            }
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full();
                    let frame_el = div()
                        .id("cua-frame")
                        .relative()
                        .w(px(logical_w))
                        .h(px(logical_h))
                        .child(gpui::img(path).w(px(logical_w)).h(px(logical_h)))
                        .child(recorder)
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                                let wx: f32 = ev.position.x.into();
                                let wy: f32 = ev.position.y.into();
                                let (ox, oy) = origin_click.lock().map(|s| *s).unwrap_or((0., 0.));
                                let x = (wx - ox).max(0.) as f64;
                                let y = (wy - oy).max(0.) as f64;
                                let client_cell = this.client.clone();
                                let thread = click_thread.clone();
                                cx.spawn(async move |_this, cx: &mut AsyncApp| {
                                    let cell = client_cell.clone();
                                    if let Some(client) = cx
                                        .background_executor()
                                        .spawn(async move { ensure_client(&cell) })
                                        .await
                                    {
                                        let thread2 = thread.clone();
                                        let _ = cx
                                            .background_executor()
                                            .spawn(async move {
                                                crate::cua::click(&client, &thread2, x, y, "left")
                                            })
                                            .await;
                                    }
                                })
                                .detach();
                            }),
                        );
                    cua_surface = cua_surface.child(
                        div()
                            .id("cua-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p_2()
                            .child(frame_el),
                    );
                } else {
                    cua_surface = cua_surface.child(
                        div()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .flex()
                            .text_color(theme.muted_foreground)
                            .text_size(px(13.))
                            .child(if connected {
                                "waiting for the first frame…"
                            } else {
                                "the desktop stream is not connected"
                            }),
                    );
                }
                main = main.child(cua_surface);
            } else if self.view_terminal && self.term_active {
                main = main.child(term_surface);
            } else if self.split_open {
                // Zed-style split: two ratio-sized panes + draggable divider.
                let ratio = self.split_ratio.clamp(0.25, 0.75);
                let left = v_flex()
                    .relative()
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
                    .child(div().flex_1().min_h_0().child(
                        pane,
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
                    .child(div().flex_1().min_h_0().child(split_pane))
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
                // Status banners anchor the top of the chat column, ahead of
                // the transcript scroll (upstream ChatColumnBannerFrame order:
                // provider health, then the thread error).
                if let Some(pb) = provider_banner {
                    main = main.child(pb);
                }
                if let Some(tb) = thread_error_banner {
                    main = main.child(tb);
                }
                main = main.child(div().flex_1().min_h_0().child(
                    pane,
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
                            this.changes_diff_state = None;
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
        if let Some((dpath, _patch)) = self.changes_diff.clone() {
            if let Some(dstate) = self.changes_diff_state.clone() {
                let _ = dpath;
                viewport = viewport.child(
                    gpui_kit::component::diff::Diff::new(&dstate)
                        .line_number(true)
                        .syntax_highlight(true)
                        .header_visible(false)
                        .hunk_separator(gpui_kit::component::diff::DiffHunkSeparator::Metadata)
                        .hover_highlight(gpui_kit::component::diff::DiffHoverHighlight::Line)
                        .on_line_click({
                            let ent = cx.entity();
                            move |pos: &gpui_kit::component::diff::DiffLinePosition,
                                  ev: &gpui::ClickEvent,
                                  _w,
                                  cx| {
                                let p = pos.path().to_string();
                                let ln = pos.line() as u32;
                                let base = matches!(
                                    pos.side(),
                                    gpui_kit::component::diff::DiffSide::Original
                                );
                                let (px_, py_) = (
                                    ev.position().x.as_f32(),
                                    ev.position().y.as_f32(),
                                );
                                let _ = ent.update(cx, |this, cx| {
                                    this.fetch_blame(p, ln, base, px_, py_, cx);
                                });
                            }
                        }),
                );
            }
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
                .my(px(2.))
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
        let desktop_update = self
            .desktop_update_state
            .clone()
            .or_else(|| crate::updater::desktop_update_service().snapshot_state())
            .filter(|state| crate::updater::should_show_desktop_update_button(Some(state)));
        let rail = v_flex()
            .w(px(51.))
            .h_full()
            .pt(px(10.))
            .pb(px(10.))
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
            .when_some(desktop_update, |rail, update_state| {
                let percent = crate::updater::desktop_update_download_percent(&update_state);
                let disabled = crate::updater::desktop_update_button_disabled(Some(&update_state))
                    || self.desktop_update_installing;
                let label = crate::updater::desktop_update_button_label(&update_state, self.desktop_update_installing);
                rail.child(
                    Button::new("nav-update")
                        .ghost()
                        .compact()
                        .w(px(36.))
                        .h(px(36.))
                        .p_0()
                        .rounded(px(8.))
                        .disabled(disabled)
                        .accessibility_label(label.clone())
                        .tooltip(label)
                        .child(
                            div()
                                .size(px(28.))
                                .rounded_full()
                                .bg(theme.primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    if let Some(percent) = percent {
                                        div()
                                            .text_size(px(9.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.primary_foreground)
                                            .child(format!("{percent}%"))
                                            .into_any_element()
                                    } else {
                                        svg_icon(icons::DOWNLOAD)
                                            .size(px(16.))
                                            .text_color(theme.primary_foreground)
                                            .into_any_element()
                                    },
                                ),
                        )
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.perform_desktop_update_action(cx);
                        })),
                )
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
        let route_view = self.view_inbox
            || self.view_tasks
            || self.view_kanban
            || self.view_autom
            || self.view_pullreq
            || self.view_studio;
        // 47px #f7f7f7 band, borderless, over rail+panel+content: sidebar
        // toggle centered over the rail at left, Environment + dock toggles
        // at right (Hugeicons panel-top-open / layout-right).
        // Upstream top strip is transparent chrome over the shell atmosphere
        // (part of the frame clip), not an opaque band.
        let strip = h_flex()
            .w_full()
            .h(px(47.))
            .items_center()
            .relative()
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
                    .w(if self.sidebar_collapsed { px(0.) } else { px(self.side_panel_w()) })
                    .flex_none(),
            )
            // Upstream rail routes have no thread tabs in the top bar.
            .when(!route_view, |d| {
                d.child({
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
                        let ctx_tid = ot.clone();
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
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                                    this.ctx_menu = Some((
                                        ctx_tid.clone(),
                                        ev.position.x.as_f32(),
                                        ev.position.y.as_f32(),
                                    ));
                                    this.ctx_menu_tab = true;
                                    this.ctx_sub = None;
                                    this.ctx_confirm_delete = None;
                                    this.hover_card = None;
                                    this.hover_thread = None;
                                    this.hover_card_seq += 1;
                                    cx.notify();
                                    cx.stop_propagation();
                                }),
                            )
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
            })
            .child(div().flex_1())
            // Upstream rail routes (inbox, tasks, kanban, code review, automations,
            // hubs) show no thread controls at the top right.
            .when(!route_view, |d| {
                d
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
            )
            });
        // Tasks/kanban routes: upstream puts the List|Kanban segmented
        // control in the top strip at the right end.
        let strip = if self.view_tasks || self.view_kanban {
            let chip = |id: &'static str, label: &'static str, active: bool| {
                div()
                    .id(ElementId::Name(id.into()))
                    .px(px(10.))
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_size(px(12.))
                    .text_color(if active { theme.foreground } else { theme.muted_foreground })
                    .when(active, |d| d.bg(theme.foreground.opacity(0.06)))
                    .child(label)
            };
            // Upstream strip rows (KanbanView.tsx / TasksView.tsx headers):
            // kanban — [Kanban title ui-lg medium] [N tasks muted] [List|Kanban]
            // [Attention|Classic] … [+ New task chrome button ml-auto]; list —
            // only the List|Kanban switch, right-aligned. Attention/Classic are
            // a documented feature gap: their chips fall back to the kanban view.
            if self.view_kanban {
                strip.child(
                    h_flex()
                        .absolute()
                        .inset_0()
                        .w_full()
                        .items_center()
                        .gap(px(8.))
                        .pl(px(78.))
                        .pr(px(10.))
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .child("Kanban"),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(theme.muted_foreground)
                                .child(format!("{} tasks", kanban_count)),
                        )
                        .child(
                            chip("strip-seg-list", "List", false)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(1, cx))),
                        )
                        .child(
                            chip("strip-seg-kanban", "Kanban", true)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
                        )
                        .child(div().w(px(2.)))
                        .child(
                            chip("strip-seg-attention", "Attention", false)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
                        )
                        .child(
                            chip("strip-seg-classic", "Classic", false)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
                        )
                        .child(div().flex_1())
                        .child(
                            h_flex()
                                .id("strip-new-task")
                                .items_center()
                                .gap(px(6.))
                                .px(px(8.))
                                .h(px(24.))
                                .rounded(px(6.))
                                .cursor_pointer()
                                .child(
                                    svg_icon(icons::PLUS)
                                        .size(px(14.))
                                        .text_color(theme.foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.foreground)
                                        .child("New task"),
                                )
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(1, cx))),
                        ),
                )
            } else {
                strip.child(
                    h_flex()
                        .absolute()
                        .inset_0()
                        .w_full()
                        .items_center()
                        .justify_end()
                        .gap(px(4.))
                        .pr(px(10.))
                        .child(
                            chip("strip-seg-list", "List", true)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(1, cx))),
                        )
                        .child(
                            chip("strip-seg-kanban", "Kanban", false)
                                .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
                        ),
                )
            }
        } else {
            strip
        };

        // ---- left column: #f7f7f7 well holding the rail and the floating
        // white sidebar card (bordered right+bottom, rounded corners, ~4px
        // bottom inset) exactly like upstream's app-rail-panel.
        let well_bg = if self.backdrop_full && self.show_ascii_bg {
            rgba(0x00000000)
        } else {
            well_bg
        };
        // Upstream rail strip is transparent over the shell atmosphere; the
        // sidebar card paints its own fill, so the well itself stays clear.
        let mut left_well = h_flex()
            .h_full()
            .child(rail.border_r_1().border_color(hairline));
        if !self.sidebar_collapsed
            && !self.view_inbox
            && !self.view_tasks
            && !self.view_kanban
        {
            let side: gpui::AnyElement = if self.view_settings {
                settings_nav_el
            } else if self.view_pullreq {
                pullreq_panel
            } else if self.view_autom {
                autom_panel
            } else if self.view_studio {
                studio_panel
            } else {
                sidebar
            };
            left_well = left_well.child(
                v_flex()
                    .w(px(self.side_panel_w()))
                    .h_full()
                    .mb(px(4.))
                    .bg(if (self.backdrop_full || self.landing_active()) && self.show_ascii_bg {
                        // Empty-chat landing and full-window atmosphere: the
                        // floating sidebar card goes clear so the texture runs
                        // behind it (upstream sidebar over EmptyChatBackdrop).
                        gpui::transparent_black()
                    } else {
                        sidebar_bg
                    })
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
                    .bg(theme.primary)
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

        let mut body = h_flex().flex_1().min_h_0().relative();
        // (atmosphere mounts at the root below, behind strip + body)

        body = body.child(left_well).child(main);
        if self.view_changes {
            body = body.child(changes_pane.border_l_1().border_color(theme.border));
        }

        let overlay_parts = self.build_overlays(window, cx);
        let overlays::NativeOverlays {
            ctx_overlay,
            hc_overlay,
            pc_overlay,
            mp_overlay,
            bp_overlay,
            ren_overlay,
            palette_overlay,
            sc_overlay,
            fb_overlay,
            scd_overlay,
            snz_overlay,
            tour_overlay,
            git_overlay,
            autom_overlay,
            tp_overlay,
            imp_overlay,
            pr_overlay,
            wn_pop_overlay,
            wn_dialog_overlay
        } = overlay_parts;

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
                        x0 += this.side_panel_w();
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
            .on_action(cx.listener(|this, _: &ToggleBrowser, w, cx| {
                this.toggle_browser(w, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleComputer, w, cx| {
                this.toggle_cua(w, cx);
            }))
            .bg(content_bg)
            .text_color(theme.foreground)
            // Upstream .app-shell-atmosphere: the texture always prints
            // behind the window chrome. Default mode shows it in the frame
            // strips (top strip, rail, thin bottom/right inset) at a third of
            // the landing weight (35% light / 80% dark); data-atmosphere-full
            // opens the clip to the whole window at full strength. Opaque
            // content surfaces cover the center either way.
            .children(atmosphere_layer.take().map(|atmo| {
                let frame_op = if self.backdrop_full {
                    1.0
                } else if dark_ui {
                    0.8
                } else {
                    0.35
                };
                div()
                    .absolute()
                    .inset_0()
                    .overflow_hidden()
                    .opacity(frame_op)
                    .child(atmo)
            }))
            .children(landing_bd_root)
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
            .when(self.model_picker_open, |d| d.child(mp_overlay))
            .when(self.blame_pop.is_some(), |d| d.child(bp_overlay))
            .when(
                self.renaming || self.proj_rename.is_some() || self.proj_relocate.is_some(),
                |d| d.child(ren_overlay),
            )
            .when(self.palette_mode.is_some(), |d| d.child(palette_overlay))
            .when(self.sc_rec.is_some(), |d| d.child(sc_overlay))
            .when(self.feedback_open, |d| d.child(fb_overlay))
            .when(self.sc_dialog_open, |d| d.child(scd_overlay))
            .when(self.snooze_dlg_tid.is_some(), |d| d.child(snz_overlay))
            .when(self.tour_step.is_some(), |d| d.child(tour_overlay))
            .when(self.git_dialog.is_some(), |d| d.child(git_overlay))
            .when(self.autom_dialog.is_some(), |d| d.child(autom_overlay))
            .when(
                self.tp_edit.is_some() || self.tp_import.is_some(),
                |d| d.child(tp_overlay),
            )
            .when(self.import_dlg.is_some(), |d| d.child(imp_overlay))
            .when(self.pr_dialog_open, |d| d.child(pr_overlay))
            .when(self.wn_popout && self.wn_view == 0, |d| {
                d.child(wn_pop_overlay)
            })
            .when(self.wn_view != 0, |d| d.child(wn_dialog_overlay))
    }
}


/// Settings hydration applied to the shell state once the server-owned
/// settings document lands: appearance, power, sidebar UI mirrors, coachmarks,
/// feature flags, what's-new. Extracted from fetch_settings so the critical
/// stage can apply it without waiting for the heavy RPC tail.
pub fn native_hydrate_settings(this: &mut Native, cx: &mut Context<Native>) {
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
    this.backdrop_full = ap
        .get("atmosphereFullWindow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    this.theme_packs = get(ap, "themePacks").clone();
}
this.keep_awake_mode = match str_at(app, "keepAwakeMode") {
    "always" | "on" => 0,
    "agent" => 1,
    _ => 2,
};
// Sidebar UI state (upstream localStorage mirrors): dismissed
// status keys + last-visited stamps drive pill suppression.
let su = get(get(&this.settings_data, "settings"), "sidebarUi").clone();
this.apply_appearance_mode(None, cx);
this.apply_theme_pack(cx);
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
this.wn_seen = get(&su, "whatsNewLastSeenVersion")
    .as_str()
    .map(|s| s.to_string());
this.eval_whats_new(cx);
this.show_next_coachmark(cx);
}
