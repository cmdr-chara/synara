//! Multiple native file buffers. The active Shell fields mirror exactly one tab.
//! Each retained input owns its undo history, selection, scrolling and IME state.
use super::*;
mod autosave;
mod batch;
pub(super) mod history;
use crate::ui::{self, Glyph, palette};

pub(super) const MAX_TABS: usize = 24;
const MAX_COMPARE_LINES: usize = 6000;
const MAX_COMPARE_CELLS: usize = 2_000_000;
const MAX_COMPARE_LINE_CHARS: usize = 2000;
const MAX_MERGE_LINES: usize = 6000;
const MAX_MERGE_CELLS: usize = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompareMode {
    Saved,
    Disk,
    GitRef,
}
#[derive(Clone, PartialEq, Eq)]
struct CompareOwner {
    task: Option<TaskId>,
    project: Option<ProjectId>,
    root: PathBuf,
    path: PathBuf,
    tab: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompareLineKind {
    Same,
    Added,
    Removed,
    Info,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct CompareLine {
    old: Option<usize>,
    new: Option<usize>,
    text: String,
    kind: CompareLineKind,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CompareDiff {
    lines: Vec<CompareLine>,
    added: usize,
    removed: usize,
    limited: bool,
}
fn compare_line(
    old: Option<usize>,
    new: Option<usize>,
    text: &str,
    kind: CompareLineKind,
    limited: &mut bool,
) -> CompareLine {
    let original_chars = text.chars().count();
    let mut text: String = text.chars().take(MAX_COMPARE_LINE_CHARS).collect();
    if original_chars > MAX_COMPARE_LINE_CHARS {
        text.push_str(" [line shortened]");
        *limited = true;
    }
    CompareLine {
        old,
        new,
        text,
        kind,
    }
}
fn newline_style(text: &str) -> &'static str {
    let crlf = text.matches("\r\n").count();
    let lf = text.bytes().filter(|byte| *byte == b'\n').count() - crlf;
    match (crlf > 0, lf > 0) {
        (true, true) => "mixed",
        (true, false) => "CRLF",
        (false, true) => "LF",
        (false, false) => "no line breaks",
    }
}
fn compare_text(reference: &str, current: &str) -> CompareDiff {
    let limited_diff = || CompareDiff {
        lines: vec![CompareLine {
            old: None,
            new: None,
            text: "Comparison exceeds the safe inline diff limit. Your editor buffer is unchanged."
                .into(),
            kind: CompareLineKind::Info,
        }],
        limited: true,
        ..Default::default()
    };
    if reference.len() > MAX_EDITOR_BYTES || current.len() > MAX_EDITOR_BYTES {
        return limited_diff();
    }
    let line_count = |text: &str| {
        if text.is_empty() {
            0
        } else {
            text.bytes().filter(|byte| *byte == b'\n').count() + usize::from(!text.ends_with('\n'))
        }
    };
    if line_count(reference) > MAX_COMPARE_LINES || line_count(current) > MAX_COMPARE_LINES {
        return limited_diff();
    }

    // Compare text content independently from CRLF/LF spelling, then call out
    // line-ending differences explicitly so they do not become thousands of
    // misleading add/remove rows.
    let reference_lines: Vec<_> = reference
        .split_terminator('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let current_lines: Vec<_> = current
        .split_terminator('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let (old, new) = (&reference_lines, &current_lines);
    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len().saturating_sub(prefix)
        && suffix < new.len().saturating_sub(prefix)
        && old[old.len() - suffix - 1] == new[new.len() - suffix - 1]
    {
        suffix += 1;
    }
    let old_end = old.len() - suffix;
    let new_end = new.len() - suffix;
    let old_middle = &old[prefix..old_end];
    let new_middle = &new[prefix..new_end];
    let width = new_middle.len().saturating_add(1);
    let cells = old_middle.len().saturating_add(1).saturating_mul(width);
    if cells > MAX_COMPARE_CELLS {
        return limited_diff();
    }

    let mut lcs = vec![0u32; cells];
    for row in (0..old_middle.len()).rev() {
        for column in (0..new_middle.len()).rev() {
            let index = row * width + column;
            lcs[index] = if old_middle[row] == new_middle[column] {
                1 + lcs[(row + 1) * width + column + 1]
            } else {
                lcs[(row + 1) * width + column].max(lcs[row * width + column + 1])
            };
        }
    }

    let mut result = CompareDiff::default();
    for (index, line) in old.iter().enumerate().take(prefix) {
        result.lines.push(compare_line(
            Some(index + 1),
            Some(index + 1),
            line,
            CompareLineKind::Same,
            &mut result.limited,
        ));
    }
    let (mut row, mut column) = (0, 0);
    while row < old_middle.len() || column < new_middle.len() {
        if row < old_middle.len()
            && column < new_middle.len()
            && old_middle[row] == new_middle[column]
        {
            result.lines.push(compare_line(
                Some(prefix + row + 1),
                Some(prefix + column + 1),
                old_middle[row],
                CompareLineKind::Same,
                &mut result.limited,
            ));
            row += 1;
            column += 1;
        } else if row < old_middle.len()
            && (column == new_middle.len()
                || lcs[(row + 1) * width + column] >= lcs[row * width + column + 1])
        {
            result.lines.push(compare_line(
                Some(prefix + row + 1),
                None,
                old_middle[row],
                CompareLineKind::Removed,
                &mut result.limited,
            ));
            result.removed += 1;
            row += 1;
        } else {
            result.lines.push(compare_line(
                None,
                Some(prefix + column + 1),
                new_middle[column],
                CompareLineKind::Added,
                &mut result.limited,
            ));
            result.added += 1;
            column += 1;
        }
    }
    for index in 0..suffix {
        result.lines.push(compare_line(
            Some(old_end + index + 1),
            Some(new_end + index + 1),
            old[old_end + index],
            CompareLineKind::Same,
            &mut result.limited,
        ));
    }
    if newline_style(reference) != newline_style(current) {
        result.lines.push(CompareLine {
            old: None,
            new: None,
            text: format!(
                "Line endings differ: reference {} · buffer {}.",
                newline_style(reference),
                newline_style(current)
            ),
            kind: CompareLineKind::Info,
        });
    }
    if reference.ends_with('\n') != current.ends_with('\n') {
        result.lines.push(CompareLine {
            old: None,
            new: None,
            text: format!(
                "Final newline differs: reference {} · buffer {}.",
                if reference.ends_with('\n') {
                    "present"
                } else {
                    "absent"
                },
                if current.ends_with('\n') {
                    "present"
                } else {
                    "absent"
                }
            ),
            kind: CompareLineKind::Info,
        });
    }
    if result.lines.len() > MAX_COMPARE_LINES {
        result.lines.truncate(MAX_COMPARE_LINES);
        result.limited = true;
    }
    result
}

fn revert_compare_block(
    reference: &str,
    current: &str,
    diff: &CompareDiff,
    index: usize,
) -> Option<String> {
    if diff.limited
        || reference.contains('\r')
        || current.contains('\r')
        || reference.ends_with('\n') != current.ends_with('\n')
        || !matches!(
            diff.lines.get(index)?.kind,
            CompareLineKind::Added | CompareLineKind::Removed
        )
        || index > 0
            && matches!(
                diff.lines[index - 1].kind,
                CompareLineKind::Added | CompareLineKind::Removed
            )
    {
        return None;
    }
    let end = index
        + diff.lines[index..]
            .iter()
            .take_while(|line| {
                matches!(line.kind, CompareLineKind::Added | CompareLineKind::Removed)
            })
            .count();
    let start_line = diff.lines[..index]
        .iter()
        .filter(|line| line.new.is_some())
        .count();
    let added: Vec<_> = diff.lines[index..end]
        .iter()
        .filter(|line| line.kind == CompareLineKind::Added)
        .map(|line| line.text.as_str())
        .collect();
    let removed: Vec<_> = diff.lines[index..end]
        .iter()
        .filter(|line| line.kind == CompareLineKind::Removed)
        .map(|line| line.text.as_str())
        .collect();
    let mut lines: Vec<_> = current.split_terminator('\n').collect();
    if lines.get(start_line..start_line.checked_add(added.len())?)? != added.as_slice() {
        return None;
    }
    lines.splice(start_line..start_line + added.len(), removed);
    let mut result = lines.join("\n");
    if current.ends_with('\n') {
        result.push('\n');
    }
    Some(result)
}

fn restore_compare_all(reference: &str, current: &str, diff: &CompareDiff) -> Option<String> {
    if diff.limited || reference == current || compare_text(reference, current) != *diff {
        return None;
    }
    Some(reference.to_owned())
}

fn compare_visible_indices(diff: &CompareDiff, changes_only: bool) -> Vec<usize> {
    if !changes_only {
        return (0..diff.lines.len()).collect();
    }
    diff.lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line.kind != CompareLineKind::Same).then_some(index))
        .collect()
}

fn compare_change_starts(diff: &CompareDiff) -> Vec<usize> {
    diff.lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let changed = matches!(line.kind, CompareLineKind::Added | CompareLineKind::Removed);
            let previous_changed = index > 0
                && matches!(
                    diff.lines[index - 1].kind,
                    CompareLineKind::Added | CompareLineKind::Removed
                );
            (changed && !previous_changed).then_some(index)
        })
        .collect()
}

fn next_compare_change(
    diff: &CompareDiff,
    current: Option<usize>,
    backwards: bool,
) -> Option<usize> {
    let starts = compare_change_starts(diff);
    if starts.is_empty() {
        return None;
    }
    let position = current.and_then(|current| starts.iter().position(|index| *index == current));
    Some(match (position, backwards) {
        (Some(position), true) => starts[(position + starts.len() - 1) % starts.len()],
        (Some(position), false) => starts[(position + 1) % starts.len()],
        (None, true) => *starts.last()?,
        (None, false) => starts[0],
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LineEdit {
    start: usize,
    end: usize,
    replacement: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ThreeWayMerge {
    text: Option<String>,
    local_edits: usize,
    disk_edits: usize,
    conflicts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergeLimit {
    Bytes,
    Lines,
    Cells,
}

fn three_way_merge(base: &str, local: &str, disk: &str) -> Result<ThreeWayMerge, MergeLimit> {
    if base.len() > MAX_EDITOR_BYTES
        || local.len() > MAX_EDITOR_BYTES
        || disk.len() > MAX_EDITOR_BYTES
    {
        return Err(MergeLimit::Bytes);
    }
    let base_lines: Vec<_> = base.split_inclusive('\n').collect();
    let local_lines: Vec<_> = local.split_inclusive('\n').collect();
    let disk_lines: Vec<_> = disk.split_inclusive('\n').collect();
    if base_lines.len() > MAX_MERGE_LINES
        || local_lines.len() > MAX_MERGE_LINES
        || disk_lines.len() > MAX_MERGE_LINES
    {
        return Err(MergeLimit::Lines);
    }

    let local_edits = line_edits(&base_lines, &local_lines)?;
    let disk_edits = line_edits(&base_lines, &disk_lines)?;
    let local_edit_count = local_edits.len();
    let disk_edit_count = disk_edits.len();
    let mut combined = local_edits
        .into_iter()
        .map(|edit| (edit, true))
        .chain(disk_edits.into_iter().map(|edit| (edit, false)))
        .collect::<Vec<_>>();
    combined.sort_by(|(left, left_local), (right, right_local)| {
        (left.start, left.end, !*left_local).cmp(&(right.start, right.end, !*right_local))
    });

    let mut accepted = Vec::<(LineEdit, bool)>::new();
    let mut conflicts = 0;
    for (edit, is_local) in combined {
        let mut duplicate = false;
        for (previous, previous_local) in &accepted {
            if *previous_local == is_local || !line_edits_overlap(previous, &edit) {
                continue;
            }
            if previous == &edit {
                duplicate = true;
            } else {
                conflicts += 1;
            }
        }
        if !duplicate {
            accepted.push((edit, is_local));
        }
    }
    if conflicts > 0 {
        return Ok(ThreeWayMerge {
            text: None,
            local_edits: local_edit_count,
            disk_edits: disk_edit_count,
            conflicts,
        });
    }

    let mut merged = String::new();
    let mut cursor = 0;
    for (edit, _) in accepted {
        if edit.start < cursor || edit.end > base_lines.len() {
            return Err(MergeLimit::Cells);
        }
        for line in &base_lines[cursor..edit.start] {
            merged.push_str(line);
        }
        for line in &edit.replacement {
            merged.push_str(line);
        }
        cursor = edit.end;
    }
    for line in &base_lines[cursor..] {
        merged.push_str(line);
    }
    if merged.len() > MAX_EDITOR_BYTES {
        return Err(MergeLimit::Bytes);
    }
    if merged.split_inclusive('\n').count() > MAX_MERGE_LINES {
        return Err(MergeLimit::Lines);
    }

    Ok(ThreeWayMerge {
        text: Some(merged),
        local_edits: local_edit_count,
        disk_edits: disk_edit_count,
        conflicts: 0,
    })
}

fn line_edits(base: &[&str], modified: &[&str]) -> Result<Vec<LineEdit>, MergeLimit> {
    let width = modified.len().saturating_add(1);
    let cells = base.len().saturating_add(1).saturating_mul(width);
    if cells > MAX_MERGE_CELLS {
        return Err(MergeLimit::Cells);
    }
    let mut lcs = vec![0u32; cells];
    for row in (0..base.len()).rev() {
        for column in (0..modified.len()).rev() {
            let index = row * width + column;
            lcs[index] = if base[row] == modified[column] {
                1 + lcs[(row + 1) * width + column + 1]
            } else {
                lcs[(row + 1) * width + column].max(lcs[row * width + column + 1])
            };
        }
    }

    let mut edits = Vec::new();
    let mut active = None::<LineEdit>;
    let (mut row, mut column) = (0, 0);
    while row < base.len() || column < modified.len() {
        if row < base.len() && column < modified.len() && base[row] == modified[column] {
            if let Some(edit) = active.take() {
                edits.push(edit);
            }
            row += 1;
            column += 1;
        } else if row < base.len()
            && (column == modified.len()
                || lcs[(row + 1) * width + column] >= lcs[row * width + column + 1])
        {
            let edit = active.get_or_insert_with(|| LineEdit {
                start: row,
                end: row,
                replacement: Vec::new(),
            });
            edit.end = row + 1;
            row += 1;
        } else {
            let edit = active.get_or_insert_with(|| LineEdit {
                start: row,
                end: row,
                replacement: Vec::new(),
            });
            edit.replacement.push(modified[column].to_owned());
            column += 1;
        }
    }
    if let Some(edit) = active {
        edits.push(edit);
    }
    Ok(edits)
}

fn line_edits_overlap(left: &LineEdit, right: &LineEdit) -> bool {
    match (left.start == left.end, right.start == right.end) {
        (true, true) => left.start == right.start,
        (true, false) => left.start >= right.start && left.start <= right.end,
        (false, true) => right.start >= left.start && right.start <= left.end,
        (false, false) => left.start < right.end && right.start < left.end,
    }
}

#[derive(Default)]
struct EditorCompare {
    open: bool,
    generation: u64,
    cancel: tokio_util::sync::CancellationToken,
    pending: bool,
    owner: Option<CompareOwner>,
    mode: Option<CompareMode>,
    label: Option<String>,
    reference: Option<String>,
    disk_version: Option<synara_runtime::FileVersion>,
    buffer_at_read: Option<String>,
    diff: CompareDiff,
    error: Option<String>,
    restore_all_confirmed: bool,
    changes_only: bool,
    selected_change: Option<usize>,
}
impl EditorCompare {
    fn clear(&mut self) {
        self.cancel.cancel();
        let generation = self.generation.wrapping_add(1);
        *self = Self::default();
        self.generation = generation;
    }
}
impl Drop for EditorCompare {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

// Save errors still arrive as formatted strings; limit matching to native editor
// save prefixes and the exact active-file prefix for Save All failures.
pub(super) fn is_save_conflict(error: Option<&str>, active_path: Option<&std::path::Path>) -> bool {
    let (Some(message), Some(active_path)) = (error, active_path) else {
        return false;
    };
    let is_active_save = if let Some(failed_path) = message
        .strip_prefix("Save all stopped at ")
        .or_else(|| message.strip_prefix("Auto-save stopped at "))
    {
        let active_prefix = format!("{}: ", active_path.display());
        failed_path.starts_with(&active_prefix)
    } else {
        message.starts_with("Save failed: ") || message.starts_with("Overwrite failed: ")
    };
    is_active_save && message.contains("file changed outside this editor")
}

struct ReloadConfirmation {
    tab_id: u64,
    disk_version: synara_runtime::FileVersion,
    buffer: String,
}

fn reload_confirmation_matches(
    confirmation: &ReloadConfirmation,
    active_tab: u64,
    disk_version: &synara_runtime::FileVersion,
    buffer: &str,
    composing: bool,
) -> bool {
    confirmation.tab_id == active_tab
        && &confirmation.disk_version == disk_version
        && confirmation.buffer == buffer
        && !composing
}

pub(super) struct EditorTab {
    id: u64,
    pub(super) document: Document,
    pub(super) input: Entity<TextEntry>,
    _subscriptions: Vec<Subscription>,
}
impl EditorTab {
    fn dirty(&self, cx: &App) -> bool {
        self.input.read(cx).text() != self.document.snapshot.text
    }
}

pub(super) struct EditorState {
    pub tabs: Vec<EditorTab>,
    closed: Vec<EditorTab>,
    save_all: Option<batch::SaveProgress>,
    save_epoch: u64,
    autosave: autosave::Autosave,
    history: history::State,
    compare: EditorCompare,
    compare_ref: Entity<TextEntry>,
    compare_refs: HashMap<PathBuf, String>,
    compare_ref_root: Option<PathBuf>,
    management_open: bool,
    active: Option<u64>,
    next_id: u64,
    generation: u64,
    pub loading: Option<PathBuf>,
    close: Option<(u64, bool)>,
    reload_confirmation: Option<ReloadConfirmation>,
    pub tree_visible: bool,
    pub show_hidden: bool,
    pub preview: bool,
    pub find_open: bool,
    replace_open: bool,
    goto_open: bool,
    query: Entity<TextEntry>,
    replacement: Entity<TextEntry>,
    line: Entity<TextEntry>,
    focus_editor: bool,
    pub jump_after_open: Option<(PathBuf, usize)>,
    _subscriptions: Vec<Subscription>,
}
impl EditorState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| TextEntry::new("Find exact text", EntryMode::SingleLine, 32., cx));
        let replacement =
            cx.new(|cx| TextEntry::new("Replace with", EntryMode::SingleLine, 32., cx));
        let line = cx.new(|cx| TextEntry::new("Line:column", EntryMode::SingleLine, 32., cx));
        let compare_ref = cx.new(|cx| {
            TextEntry::new(
                "Branch, tag or commit (default HEAD)",
                EntryMode::SingleLine,
                32.,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&query, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.find_in_editor(false, cx);
                }
                cx.notify();
            }),
            cx.subscribe(&replacement, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.replace_in_editor(false, cx);
                }
                cx.notify();
            }),
            cx.subscribe(&line, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.goto_editor_line(cx);
                }
            }),
            cx.subscribe(&compare_ref, |this, input, event, cx| {
                if matches!(event, EntryEvent::Changed | EntryEvent::Submit) {
                    if let Some(root) = this.editors.compare_ref_root.clone() {
                        this.editors
                            .compare_refs
                            .insert(root, input.read(cx).text().trim().to_owned());
                    }
                    if matches!(event, EntryEvent::Submit) {
                        this.compare_editor_git_ref(cx);
                    }
                }
                cx.notify();
            }),
        ];
        Self {
            tabs: Vec::new(),
            closed: Vec::new(),
            save_all: None,
            save_epoch: 0,
            autosave: autosave::Autosave::default(),
            history: Default::default(),
            compare: Default::default(),
            compare_ref,
            compare_refs: HashMap::new(),
            compare_ref_root: None,
            management_open: false,
            active: None,
            next_id: 0,
            generation: 0,
            loading: None,
            close: None,
            reload_confirmation: None,
            tree_visible: true,
            show_hidden: true,
            preview: false,
            find_open: false,
            replace_open: false,
            goto_open: false,
            query,
            replacement,
            line,
            focus_editor: false,
            jump_after_open: None,
            _subscriptions: subscriptions,
        }
    }
    pub(super) fn sync_keybindings(
        &mut self,
        keybindings: &[synara_workspace::KeyBinding],
        cx: &mut Context<Shell>,
    ) {
        for tab in self.tabs.iter().chain(self.closed.iter()) {
            tab.input
                .update(cx, |input, _| input.set_keybindings(keybindings));
        }
    }
    pub fn dirty(&self, cx: &App) -> bool {
        self.tabs.iter().any(|tab| tab.dirty(cx))
    }
    pub fn index(&self, path: &std::path::Path) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.document.path == path)
    }
    pub fn request_open(&mut self, path: PathBuf) -> u64 {
        if self
            .jump_after_open
            .as_ref()
            .is_some_and(|(target, _)| *target != path)
        {
            self.jump_after_open = None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.loading = Some(path);
        self.generation
    }
    pub fn finish_open(&mut self, generation: u64) -> bool {
        if self.generation != generation {
            return false;
        }
        self.loading = None;
        true
    }
}

impl Shell {
    pub(super) fn restore_editor_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editors.focus_editor
            && self.panel == Panel::Files
            && !self.studio.open
            && !self.command_palette.open
            && !self.explorer.modal_open()
            && !self.controls.is_open()
            && !self.chat_tools.menu_open()
            && self.kanban.dialog.is_none()
            && self.organization.dialog.is_none()
            && self.saved_context.dialog.is_none()
            && self.settings.popup.is_none()
        {
            self.editors.focus_editor = false;
            window.focus(&self.editor.read(cx).focus_handle(cx), cx);
        }
    }
    pub(super) fn open_editor_paths(&self) -> Vec<PathBuf> {
        self.editors
            .tabs
            .iter()
            .map(|tab| tab.document.path.clone())
            .collect()
    }
    pub(super) fn editor_preview_available(&self) -> bool {
        self.document
            .as_ref()
            .and_then(|document| document.path.extension())
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "md" | "markdown" | "mdown"
                )
            })
    }
    pub(super) fn active_document_dirty(&self, cx: &App) -> bool {
        self.document
            .as_ref()
            .is_some_and(|document| self.editor.read(cx).text() != document.snapshot.text)
    }
    fn editor_compare_owner(&self) -> Option<CompareOwner> {
        Some(CompareOwner {
            task: self.selected,
            project: self.project,
            root: self.workspace_target()?.root().clone(),
            path: self.document.as_ref()?.path.clone(),
            tab: self.editors.active?,
        })
    }
    fn update_editor_compare_buffer(&mut self, current: &str) {
        let Some(owner) = self.editor_compare_owner() else {
            return;
        };
        let compare = &mut self.editors.compare;
        if !compare.open || compare.owner.as_ref() != Some(&owner) {
            return;
        }
        compare.restore_all_confirmed = false;
        compare.selected_change = None;
        if let Some(reference) = &compare.reference {
            compare.diff = compare_text(reference, current);
        }
    }
    fn revert_editor_compare_block(
        &mut self,
        index: usize,
        expected: CompareLine,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if self.saving
            || self.editor.read(cx).is_composing()
            || self.editor_compare_owner().as_ref() != self.editors.compare.owner.as_ref()
        {
            return;
        }
        let compare = &self.editors.compare;
        if !compare.open
            || compare.pending
            || compare.error.is_some()
            || compare.generation != generation
        {
            return;
        }
        let Some(reference) = &compare.reference else {
            return;
        };
        let current = self.editor.read(cx).text().to_owned();
        let diff = compare_text(reference, &current);
        if diff.lines.get(index) != Some(&expected) || diff != compare.diff {
            self.notice = Some(
                "The editor changed. Review the comparison again before reverting a block.".into(),
            );
            cx.notify();
            return;
        }
        let Some(result) = revert_compare_block(reference, &current, &diff, index) else {
            self.notice =
                Some("This block cannot be reverted safely from the current comparison.".into());
            cx.notify();
            return;
        };
        self.editor
            .update(cx, |input, cx| input.set_text(result.clone(), cx));
        self.update_editor_compare_buffer(&result);
        self.notice =
            Some("One comparison block was restored in the editor. Save to write the file.".into());
        cx.notify();
    }
    fn copy_editor_compare_block(
        &mut self,
        index: usize,
        expected: CompareLine,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if self.editor_compare_owner().as_ref() != self.editors.compare.owner.as_ref() {
            return;
        }
        let compare = &self.editors.compare;
        if !compare.open
            || compare.pending
            || compare.error.is_some()
            || compare.generation != generation
            || compare.diff.limited
        {
            return;
        }
        let Some(reference) = &compare.reference else {
            return;
        };
        let current = self.editor.read(cx).text().to_owned();
        if compare.diff.lines.get(index) != Some(&expected)
            || compare_text(reference, &current) != compare.diff
        {
            self.notice = Some("The editor changed. Compare again before copying a block.".into());
            cx.notify();
            return;
        }
        let mut block = String::new();
        for line in compare.diff.lines[index..]
            .iter()
            .take_while(|line| {
                matches!(line.kind, CompareLineKind::Added | CompareLineKind::Removed)
            })
            .take(256)
        {
            let marker = if line.kind == CompareLineKind::Added {
                '+'
            } else {
                '-'
            };
            if block.len().saturating_add(line.text.len()) > 32 * 1024 {
                block.push_str("[Block shortened]\n");
                break;
            }
            block.push(marker);
            block.push_str(&line.text);
            block.push('\n');
        }
        if !block.is_empty() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(block));
            self.notice = Some("Changed block copied for review.".into());
            cx.notify();
        }
    }
    fn select_editor_compare_change(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let compare = &mut self.editors.compare;
        if !compare.open || compare.pending || compare.error.is_some() || compare.diff.limited {
            return;
        }
        compare.selected_change =
            next_compare_change(&compare.diff, compare.selected_change, backwards);
        cx.notify();
    }

    fn copy_selected_editor_compare_block(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.editors.compare.selected_change else {
            return;
        };
        let Some(expected) = self.editors.compare.diff.lines.get(index).cloned() else {
            self.editors.compare.selected_change = None;
            cx.notify();
            return;
        };
        let generation = self.editors.compare.generation;
        self.copy_editor_compare_block(index, expected, generation, cx);
    }

    fn restore_selected_editor_compare_block(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.editors.compare.selected_change else {
            return;
        };
        let Some(expected) = self.editors.compare.diff.lines.get(index).cloned() else {
            self.editors.compare.selected_change = None;
            cx.notify();
            return;
        };
        let generation = self.editors.compare.generation;
        self.revert_editor_compare_block(index, expected, generation, cx);
    }

    fn restore_all_editor_compare_changes(&mut self, generation: u64, cx: &mut Context<Self>) {
        if self.saving
            || self.editor.read(cx).is_composing()
            || self.editor_compare_owner().as_ref() != self.editors.compare.owner.as_ref()
        {
            return;
        }
        let current = self.editor.read(cx).text().to_owned();
        let (result, confirmed) = {
            let compare = &self.editors.compare;
            if !compare.open
                || compare.pending
                || compare.error.is_some()
                || compare.generation != generation
                || compare.diff.limited
            {
                return;
            }
            let Some(reference) = &compare.reference else {
                return;
            };
            (
                restore_compare_all(reference, &current, &compare.diff),
                compare.restore_all_confirmed,
            )
        };
        let Some(result) = result else {
            self.editors.compare.restore_all_confirmed = false;
            self.notice = Some(
                "The editor changed. Review the comparison again before restoring all changes."
                    .into(),
            );
            cx.notify();
            return;
        };
        if !confirmed {
            self.editors.compare.restore_all_confirmed = true;
            self.notice = Some(
                "Restore all is ready. Choose it again to replace this unsaved buffer with the comparison reference."
                    .into(),
            );
            cx.notify();
            return;
        }

        self.editors.compare.restore_all_confirmed = false;
        self.editor
            .update(cx, |input, cx| input.set_text(result.clone(), cx));
        self.update_editor_compare_buffer(&result);
        self.notice = Some(
            "All comparison changes were restored in the editor. Save to write the file.".into(),
        );
        cx.notify();
    }

    fn close_editor_compare(&mut self) {
        self.editors.compare.clear();
    }
    pub(super) fn conflict_reload_confirmed(&self, cx: &App) -> bool {
        let (Some(confirmation), Some(active), Some(document)) = (
            self.editors.reload_confirmation.as_ref(),
            self.editors.active,
            self.document.as_ref(),
        ) else {
            return false;
        };
        reload_confirmation_matches(
            confirmation,
            active,
            &document.snapshot.version,
            self.editor.read(cx).text(),
            self.editor.read(cx).is_composing(),
        )
    }
    pub(super) fn clear_conflict_reload_confirmation(&mut self) {
        self.editors.reload_confirmation = None;
    }
    fn conflict_merge_ready(&self, cx: &App) -> bool {
        let Some(owner) = self.editor_compare_owner() else {
            return false;
        };
        let compare = &self.editors.compare;
        is_save_conflict(
            self.error.as_deref(),
            self.document
                .as_ref()
                .map(|document| document.path.as_path()),
        ) && !self.editor_batch_blocked(cx)
            && compare.open
            && !compare.pending
            && compare.mode == Some(CompareMode::Disk)
            && compare.owner.as_ref() == Some(&owner)
            && compare.reference.is_some()
            && compare.disk_version.is_some()
            && compare.buffer_at_read.as_deref() == Some(self.editor.read(cx).text())
            && !self.editor.read(cx).is_composing()
    }
    pub(super) fn reset_editor_tabs(&mut self) {
        self.editors.history.clear();
        self.editors.compare.clear();
        self.editors.tabs.clear();
        self.editors.closed.clear();
        if self.editors.autosave.writing {
            self.saving = false;
        }
        self.editors.autosave = autosave::Autosave::default();
        self.editors.save_epoch = self.editors.save_epoch.wrapping_add(1);
        if self.editors.save_all.take().is_some() {
            self.saving = false;
        }
        self.editors.management_open = false;
        self.editors.active = None;
        self.editors.close = None;
        self.editors.reload_confirmation = None;
        self.editors.generation = self.editors.generation.wrapping_add(1);
        self.editors.loading = None;
        self.editors.jump_after_open = None;
        self.editors.preview = false;
        self.editors.focus_editor = false;
    }
    pub(super) fn sync_editor_document(&mut self) {
        if let Some(document) = &self.document
            && let Some(tab) = self
                .editors
                .tabs
                .iter_mut()
                .find(|tab| Some(tab.id) == self.editors.active)
        {
            tab.document = document.clone();
        }
    }
    pub(super) fn install_editor_document(&mut self, document: Document, cx: &mut Context<Self>) {
        if let Some(index) = self.editors.index(&document.path) {
            self.activate_editor(index, cx);
            return;
        }
        if self.editors.tabs.len() >= MAX_TABS {
            self.notice = Some(
                "The file is available in Explorer. Close a tab to open another buffer.".into(),
            );
            return;
        }
        self.editors
            .closed
            .retain(|tab| tab.document.path != document.path);
        let keybindings = self.settings.value.keybindings.clone();
        let syntax_path = document.path.clone();
        let input = cx.new(|cx| {
            let mut input = TextEntry::new("", EntryMode::Editor, 480., cx);
            input.set_keybindings(&keybindings);
            input.set_syntax_from_path(&syntax_path);
            input.set_text(document.snapshot.text.clone(), cx);
            input
        });
        let subscriptions = vec![
            cx.subscribe(&input, |this, input, event, cx| {
                if input.entity_id() == this.editor.entity_id() && matches!(event, EntryEvent::Save)
                {
                    this.save_file(cx);
                }
                if matches!(event, EntryEvent::Changed) {
                    this.update_editor_compare_buffer(input.read(cx).text());
                    this.editor_autosave_changed(input.entity_id());
                }
                cx.notify();
            }),
            cx.observe(&input, |_, _, cx| cx.notify()),
        ];
        self.editors.next_id = self.editors.next_id.wrapping_add(1);
        self.editors.tabs.push(EditorTab {
            id: self.editors.next_id,
            document,
            input,
            _subscriptions: subscriptions,
        });
        self.activate_editor(self.editors.tabs.len() - 1, cx);
    }
    pub(super) fn activate_editor(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.saving || self.explorer.modal_open() || self.editor.read(cx).is_composing() {
            return;
        }
        self.sync_editor_document();
        let Some(tab) = self.editors.tabs.get(index) else {
            return;
        };
        self.editors.history.clear();
        self.editors.compare.clear();
        self.editors.active = Some(tab.id);
        self.editor = tab.input.clone();
        self.document = Some(tab.document.clone());
        self.editors.generation = self.editors.generation.wrapping_add(1);
        self.editors.loading = None;
        self.editors.close = None;
        self.editors.reload_confirmation = None;
        self.focus_composer = false;
        self.editors.focus_editor = true;
        if let Some((path, line)) = self.editors.jump_after_open.as_ref()
            && self
                .document
                .as_ref()
                .is_some_and(|document| document.path == *path)
        {
            let line = *line;
            self.editors.jump_after_open = None;
            self.editor.update(cx, |input, cx| {
                input.go_to_line(line, 1, cx);
            });
        }
        cx.notify();
    }
    pub(super) fn replace_editor_document(&mut self, document: Document, cx: &mut Context<Self>) {
        self.editors.reload_confirmation = None;
        self.editors.compare.clear();
        if let Some(tab) = self
            .editors
            .tabs
            .iter_mut()
            .find(|tab| Some(tab.id) == self.editors.active)
        {
            tab.input.update(cx, |input, cx| {
                input.set_text(document.snapshot.text.clone(), cx)
            });
            tab.document = document.clone();
            self.document = Some(document);
        } else {
            self.install_editor_document(document, cx);
        }
    }
    pub(super) fn remove_active_editor(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self
            .editors
            .tabs
            .iter()
            .position(|tab| Some(tab.id) == self.editors.active)
        else {
            self.document = None;
            return;
        };
        self.sync_editor_document();
        let tab = self.editors.tabs.remove(index);
        self.editors.compare.clear();
        self.retain_closed_editor(tab, cx);
        self.editors.active = None;
        self.document = None;
        self.editors.close = None;
        if !self.editors.tabs.is_empty() {
            self.activate_editor(index.min(self.editors.tabs.len() - 1), cx);
        }
        cx.notify();
    }
    pub(super) fn discard_active_document(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        if let Some(document) = &self.document {
            self.editor.update(cx, |input, cx| {
                input.set_text(document.snapshot.text.clone(), cx)
            });
        }
        cx.notify();
    }
    pub(super) fn reveal_dirty_editor(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.active_document_dirty(cx) {
            return;
        }
        if let Some(index) = self.editors.tabs.iter().position(|tab| tab.dirty(cx)) {
            self.activate_editor(index, cx);
        }
    }
    fn request_editor_close(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.saving || self.explorer.modal_open() || self.editor.read(cx).is_composing() {
            return;
        }
        let Some(index) = self.editors.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        self.activate_editor(index, cx);
        if self.active_document_dirty(cx) {
            self.editors.close = Some((id, false));
        } else {
            self.remove_active_editor(cx);
        }
        cx.notify();
    }
    fn save_and_close_editor(&mut self, cx: &mut Context<Self>) {
        if let Some((id, _)) = self.editors.close
            && self.editors.active == Some(id)
        {
            self.editors.close = Some((id, true));
            self.save_file(cx);
        }
    }
    pub(super) fn finish_editor_close(&mut self, cx: &mut Context<Self>) {
        if let Some((id, true)) = self.editors.close
            && self.editors.active == Some(id)
            && !self.active_document_dirty(cx)
        {
            self.remove_active_editor(cx);
        }
    }
    pub(super) fn cycle_editor(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.editors.tabs.is_empty() {
            return;
        }
        let index = self
            .editors
            .tabs
            .iter()
            .position(|tab| Some(tab.id) == self.editors.active)
            .unwrap_or(0);
        let count = self.editors.tabs.len();
        self.activate_editor(
            if backwards {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            },
            cx,
        );
    }
    pub(super) fn open_editor_find(
        &mut self,
        replace: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document.is_none() || self.editor.read(cx).is_composing() {
            return;
        }
        self.editors.find_open = true;
        self.editors.replace_open = replace;
        self.editors.preview = false;
        let selected = self.editor.read(cx).selected_text().to_owned();
        if !selected.is_empty() && !selected.contains(['\r', '\n']) && selected.len() <= 4096 {
            self.editors
                .query
                .update(cx, |input, cx| input.set_text(selected, cx));
        }
        window.focus(&self.editors.query.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn find_in_editor(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let query = self.editors.query.read(cx).text().to_owned();
        let found = self
            .editor
            .update(cx, |input, cx| input.find_literal(&query, backwards, cx));
        if !found && !query.is_empty() {
            self.notice = Some("No exact match in the current file.".into());
        }
        cx.notify();
    }
    fn replace_in_editor(&mut self, all: bool, cx: &mut Context<Self>) {
        if self.document.is_none() || self.saving {
            return;
        }
        let query = self.editors.query.read(cx).text().to_owned();
        let replacement = self.editors.replacement.read(cx).text().to_owned();
        let count = self.editor.update(cx, |input, cx| {
            input.replace_literal(&query, &replacement, all, cx)
        });
        self.notice = Some(format!(
            "Replaced {count} matches in the editor. Save to write the file."
        ));
        cx.notify();
    }
    pub(super) fn open_editor_goto(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.document.is_none() || self.editor.read(cx).is_composing() {
            return;
        }
        self.editors.goto_open = true;
        self.editors.preview = false;
        window.focus(&self.editors.line.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn goto_editor_line(&mut self, cx: &mut Context<Self>) {
        let value = self.editors.line.read(cx).text().trim().to_owned();
        let (line, column) = value.split_once(':').unwrap_or((&value, "1"));
        let valid = match (line.parse::<usize>(), column.parse::<usize>()) {
            (Ok(line), Ok(column)) => self
                .editor
                .update(cx, |input, cx| input.go_to_line(line, column, cx)),
            _ => false,
        };
        if valid {
            self.editors.goto_open = false;
            self.editors.focus_editor = true;
        } else {
            self.notice =
                Some("Enter an existing line number, optionally followed by :column.".into());
        }
        cx.notify();
    }
    pub(super) fn editor_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.panel != Panel::Files
            || self.studio.open
            || event.prefer_character_input
            || event.is_held
            || self.editor.read(cx).is_composing()
            || self.editors.query.read(cx).is_composing()
            || self.editors.replacement.read(cx).is_composing()
            || self.editors.line.read(cx).is_composing()
            || self.controls.is_open()
            || self.chat_tools.menu_open()
            || self.navigation.menu_open
            || self.environment.menu_open()
            || self.settings.popup.is_some()
            || self
                .terminal_view
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        {
            return false;
        }
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.control || modifiers.platform;
        match event.keystroke.key.as_str() {
            "s" if command && modifiers.shift && !modifiers.alt => self.save_all_editors(cx),
            "t" if command && modifiers.shift && !modifiers.alt => self.reopen_closed_editor(cx),
            "pageup" if modifiers.alt && !command => self.move_editor_tab(true, cx),
            "pagedown" if modifiers.alt && !command => self.move_editor_tab(false, cx),
            "f" if command && !modifiers.alt && !modifiers.shift => {
                self.open_editor_find(false, window, cx)
            }
            "h" if command && !modifiers.alt && !modifiers.shift => {
                self.open_editor_find(true, window, cx)
            }
            "g" if command && !modifiers.alt && !modifiers.shift => {
                self.open_editor_goto(window, cx)
            }
            "tab" if command && !modifiers.alt => {
                self.cycle_editor(modifiers.shift, cx);
                window.focus(&self.editor.read(cx).focus_handle(cx), cx);
            }
            "w" if command && !modifiers.alt && !modifiers.shift => {
                if let Some(id) = self.editors.active {
                    self.request_editor_close(id, cx);
                }
            }
            "f3" if !command && !modifiers.alt && self.editors.find_open => {
                self.find_in_editor(modifiers.shift, cx)
            }
            "escape"
                if !command
                    && !modifiers.alt
                    && !modifiers.shift
                    && (self.editors.find_open
                        || self.editors.goto_open
                        || self.editors.close.is_some()) =>
            {
                self.editors.find_open = false;
                self.editors.goto_open = false;
                self.editors.close = None;
                window.focus(&self.editor.read(cx).focus_handle(cx), cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }
    pub(super) fn editor_tabs(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .child(
                        div()
                            .id("editor-tabs")
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .overflow_x_scroll()
                            .children(self.editors.tabs.iter().enumerate().map(|(index, tab)| {
                                let id = tab.id;
                                let active = self.editors.active == Some(id);
                                let dirty = tab.dirty(cx);
                                let name = tab
                                    .document
                                    .path
                                    .file_name()
                                    .map_or_else(String::new, |name| {
                                        name.to_string_lossy().into_owned()
                                    });
                                div()
                                    .id(("editor-tab", index))
                                    .group("editor-tab-actions")
                                    .flex()
                                    .items_center()
                                    .flex_shrink_0()
                                    .max_w(px(230.))
                                    .border_r_1()
                                    .border_color(rgb(palette().border))
                                    .when(active, |el| {
                                        el.bg(ui::surface(palette().overlay))
                                            .border_b_2()
                                            .border_color(rgb(palette().focus))
                                    })
                                    .child(
                                        ui::action(
                                            ("editor-select", index),
                                            format!("{name}{}", if dirty { " *" } else { "" }),
                                            Some(Glyph::Files),
                                            false,
                                            cx.listener(move |this, _: &(), window, cx| {
                                                if let Some(index) = this
                                                    .editors
                                                    .tabs
                                                    .iter()
                                                    .position(|tab| tab.id == id)
                                                {
                                                    this.activate_editor(index, cx);
                                                }
                                                window.focus(
                                                    &this.editor.read(cx).focus_handle(cx),
                                                    cx,
                                                );
                                            }),
                                        )
                                        .h(px(ui::row_height() + 2.))
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .rounded_none()
                                        .bg(gpui::rgba(0))
                                        .aria_label(
                                            format!(
                                                "{}{}",
                                                tab.document.path.display(),
                                                if dirty { ", unsaved" } else { "" }
                                            ),
                                        ),
                                    )
                                    .child(
                                        ui::button_shell(
                                            ("editor-close", index),
                                            "Close file",
                                            false,
                                        )
                                        .size(px(22.))
                                        .p_0()
                                        .rounded_none()
                                        .bg(gpui::rgba(0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when(!active, |el| {
                                            el.opacity(0.35)
                                                .group_hover("editor-tab-actions", |style| {
                                                    style.opacity(1.)
                                                })
                                        })
                                        .focus_visible(|style| style.opacity(1.))
                                        .child(ui::icon(Glyph::Close))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.request_editor_close(id, cx)
                                            }),
                                        ),
                                    )
                            })),
                    )
                    .child(ui::chrome_button(
                        "editor-save-all-icon",
                        "Save all open files (Ctrl/Cmd+Shift+S)",
                        Glyph::Check,
                        self.saving,
                        cx.listener(|this, _: &(), _, cx| this.save_all_editors(cx)),
                    ))
                    .child(ui::chrome_button(
                        "editor-tab-actions",
                        "Open buffer actions",
                        Glyph::More,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.editors.management_open = !this.editors.management_open;
                            cx.notify();
                        }),
                    )),
            )
            .children(
                self.editors
                    .management_open
                    .then(|| self.editor_management(cx)),
            )
            .children(self.editors.save_all.as_ref().map(|progress| {
                div()
                    .px_3()
                    .py_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child(format!(
                                "Saved {} / {} files",
                                progress.completed, progress.total
                            )),
                    )
                    .child(
                        ui::action(
                            "editor-save-all-stop",
                            "Stop after this file",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.stop_editor_saves(cx)),
                        )
                        .text_size(px(11.)),
                    )
            }))
            .into_any_element()
    }
    pub(super) fn editor_tools(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let reload_confirmed = self.conflict_reload_confirmed(cx);
        let merge_ready = self.conflict_merge_ready(cx);
        let markdown = self
            .document
            .as_ref()
            .and_then(|document| document.path.extension())
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "md" | "markdown" | "mdown"
                )
            });
        div().flex().flex_col().gap_2().flex_shrink_0()
            .child(div().flex().items_center().flex_wrap().gap_1()
                .child(ui::chrome_button("editor-find", "Find in file (Ctrl/Cmd+F)", Glyph::Search, false,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_find(false, window, cx))))
                .child(ui::action("editor-replace", "Replace", None, self.editors.replace_open && self.editors.find_open,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_find(true, window, cx))).text_size(px(11.)))
                .child(ui::action("editor-goto", "Go to line", None, self.editors.goto_open,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_goto(window, cx))).text_size(px(11.)))
                .children(markdown.then(|| ui::action("editor-preview", if self.editors.preview { "Edit Markdown" } else { "Preview Markdown" }, Some(Glyph::Notebook), self.editors.preview,
                    cx.listener(|this, _: &(), _, cx| { this.editors.preview = !this.editors.preview; cx.notify(); })).text_size(px(11.))))
                .child(ui::action("editor-autosave", if self.autosave_enabled() { "Auto-save on" } else { "Auto-save off" }, None, self.autosave_enabled(),
                    cx.listener(|this, _: &(), _, cx| this.toggle_editor_autosave(cx))).text_size(px(11.)).relative().child(ui::layout_probe("editor-autosave")))
                .child(ui::button("editor-history", "File history", false).text_size(px(11.)).relative().child(ui::layout_probe("editor-history"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.editors.compare.clear();
                        this.open_editor_history(cx);
                    })))
                .child(ui::action("editor-compare-toggle", if self.editors.compare.open { "Close compare" } else { "Compare" }, None, self.editors.compare.open,
                    cx.listener(|this, _: &(), _, cx| this.toggle_editor_comparison(cx))).text_size(px(11.)))
                .child(self.editor_actions(cx)))
            .child(self.editor_history_toolbar(cx))
            .children(self.editor_compare_panel(cx))
            .child(self.inline_comments_panel(cx))
            .children((
                self.active_document_dirty(cx)
                    && is_save_conflict(
                        self.error.as_deref(),
                        self.document.as_ref().map(|document| document.path.as_path()),
                    )
            )
            .then(|| {
                div()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_2()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .rounded_md()
                    .bg(rgb(palette().notice_surface))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.))
                            .child(if reload_confirmed {
                                "The file changed on disk. Reloading discards this buffer; confirm only if that is intended."
                            } else if merge_ready {
                                "Disk changes are reviewed. You can apply a three-way merge for non-overlapping edits; the file will not be written until you save."
                            } else {
                                "Save conflict: the file changed on disk. Your edits are intact."
                            }),
                    )
                    .child(ui::action(
                        "editor-conflict-review-disk",
                        if self.editors.compare.mode == Some(CompareMode::Disk)
                            && self.editors.compare.pending
                        {
                            "Reading disk…"
                        } else {
                            "Review disk changes"
                        },
                        None,
                        self.editors.compare.mode == Some(CompareMode::Disk)
                            && self.editors.compare.pending,
                        cx.listener(|this, _: &(), _, cx| this.compare_editor_disk(cx)),
                    ).relative().child(ui::layout_probe("editor-conflict-review-disk")))
                    .children(merge_ready.then(|| ui::action(
                        "editor-conflict-merge",
                        "Apply non-overlapping merge",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| this.merge_conflicted_editor(cx)),
                    ).relative().child(ui::layout_probe("editor-conflict-merge"))))
                    .child(ui::action(
                        "editor-conflict-reload",
                        if reload_confirmed {
                            "Confirm reload and discard"
                        } else {
                            "Reload from disk"
                        },
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.resolve_conflict_reload(cx)
                        }),
                    ).relative().child(ui::layout_probe("editor-conflict-reload")))
                    .when(reload_confirmed, |el| {
                        el.child(ui::action(
                            "editor-conflict-reload-cancel",
                            "Cancel",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.editors.reload_confirmation = None;
                                cx.notify();
                            }),
                        ))
                    })
                    .child(ui::action(
                        "editor-conflict-overwrite",
                        "Overwrite disk with this buffer",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.overwrite_conflicted_editor(cx)
                        }),
                    ).relative().child(ui::layout_probe("editor-conflict-overwrite")))
            }))
            .children(self.editors.find_open.then(|| {
                let query = self.editors.query.read(cx).text();
                let count = if query.is_empty() { 0 } else { self.editor.read(cx).text().match_indices(query).count() };
                div().flex().flex_col().gap_1()
                    .child(div().flex().items_center().gap_1()
                        .child(div().flex_1().min_w_0().child(self.editors.query.clone()))
                        .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{count}")))
                        .child(ui::chrome_button("editor-find-prev", "Previous exact match", Glyph::Back, false, cx.listener(|this, _: &(), _, cx| this.find_in_editor(true, cx))))
                        .child(ui::chrome_button("editor-find-next", "Next exact match", Glyph::Forward, false, cx.listener(|this, _: &(), _, cx| this.find_in_editor(false, cx))))
                        .child(ui::chrome_button("editor-find-close", "Close find", Glyph::Close, false, cx.listener(|this, _: &(), _, cx| { this.editors.find_open = false; cx.notify(); }))))
                    .children(self.editors.replace_open.then(|| div().flex().items_center().flex_wrap().gap_1()
                        .child(div().flex_1().min_w(px(120.)).child(self.editors.replacement.clone()))
                        .child(ui::action("editor-replace-one", "Replace", None, false, cx.listener(|this, _: &(), _, cx| this.replace_in_editor(false, cx))).text_size(px(11.)))
                        .child(ui::action("editor-replace-all", "Replace all", None, false, cx.listener(|this, _: &(), _, cx| this.replace_in_editor(true, cx))).text_size(px(11.)))))
                    .child(div().text_size(px(10.)).text_color(rgb(palette().muted)).child("Case-sensitive literal text. Replacements remain unsaved and support Undo."))
            }))
            .children(self.editors.goto_open.then(|| div().flex().items_center().gap_1()
                .child(div().flex_1().min_w_0().child(self.editors.line.clone()))
                .child(ui::action("editor-goto-confirm", "Go", None, false, cx.listener(|this, _: &(), window, cx| {
                    this.goto_editor_line(cx);
                    if !this.editors.goto_open { window.focus(&this.editor.read(cx).focus_handle(cx), cx); }
                })))))
            .children(self.editors.close.map(|(_, saving)| div().p_2().rounded_md().bg(rgb(palette().notice_surface)).flex().flex_col().gap_1()
                .child(div().text_size(px(12.)).child("This file has unsaved changes."))
                .child(div().flex().flex_wrap().gap_1()
                    .child(ui::action("editor-save-close", if saving { "Saving..." } else { "Save and close" }, None, false,
                        cx.listener(|this, _: &(), _, cx| this.save_and_close_editor(cx))))
                    .child(ui::action("editor-discard-close", "Discard and close", None, false, cx.listener(|this, _: &(), _, cx| {
                        if !this.saving { this.remove_active_editor(cx); }
                    })))
                    .child(ui::action("editor-keep-open", "Keep open", None, false, cx.listener(|this, _: &(), _, cx| { this.editors.close = None; cx.notify(); }))))))
            .into_any_element()
    }
    fn editor_compare_panel(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let comparison = &self.editors.compare;
        if !comparison.open || self.document.is_none() {
            return None;
        }
        let entity = cx.entity();
        let visible_indices = compare_visible_indices(&comparison.diff, comparison.changes_only);
        let row_count = visible_indices.len();
        let changes_only = comparison.changes_only;
        let change_starts = compare_change_starts(&comparison.diff);
        let selected_change = comparison
            .selected_change
            .filter(|index| change_starts.contains(index));
        let selected_position = selected_change
            .and_then(|selected| change_starts.iter().position(|index| *index == selected))
            .map(|position| position + 1);
        let change_navigation_available = !comparison.pending
            && comparison.error.is_none()
            && !comparison.diff.limited
            && !change_starts.is_empty();
        let restore_all_available = !comparison.pending
            && comparison.error.is_none()
            && !comparison.diff.limited
            && comparison
                .reference
                .as_ref()
                .is_some_and(|reference| reference != self.editor.read(cx).text());
        let restore_all_confirmed = comparison.restore_all_confirmed;
        let restore_all_generation = comparison.generation;
        let list = gpui::uniform_list("editor-compare-lines", row_count, move |range, _, cx| {
            entity.update(cx, |this, cx| {
                range
                    .filter_map(|visible_index| {
                        let index = *visible_indices.get(visible_index)?;
                        let line = this.editors.compare.diff.lines.get(index)?;
                        let first_change =
                            matches!(line.kind, CompareLineKind::Added | CompareLineKind::Removed)
                                && (index == 0
                                    || !matches!(
                                        this.editors.compare.diff.lines[index - 1].kind,
                                        CompareLineKind::Added | CompareLineKind::Removed
                                    ));
                        let expected = line.clone();
                        let copy_expected = expected.clone();
                        let generation = this.editors.compare.generation;
                        let selected_change = this.editors.compare.selected_change == Some(index);
                        let (marker, background, foreground) = match line.kind {
                            CompareLineKind::Same => (" ", palette().canvas, palette().text),
                            CompareLineKind::Added => {
                                ("+", palette().notice_surface, palette().focus)
                            }
                            CompareLineKind::Removed => {
                                ("−", palette().error_surface, palette().error)
                            }
                            CompareLineKind::Info => ("·", palette().overlay, palette().muted),
                        };
                        Some(
                            div()
                                .h(px(20.))
                                .w_full()
                                .flex()
                                .items_center()
                                .gap_1()
                                .bg(rgb(background))
                                .when(selected_change, |el| {
                                    el.border_l_2().border_color(rgb(palette().focus))
                                })
                                .font_family(ui::code_font())
                                .text_size(px(11.))
                                .text_color(rgb(foreground))
                                .child(format!(
                                    "{:>4} {:>4} {} ",
                                    line.old.map_or_else(|| "".into(), |n| n.to_string()),
                                    line.new.map_or_else(|| "".into(), |n| n.to_string()),
                                    marker
                                ))
                                .child(div().min_w_0().flex_1().child(line.text.clone()))
                                .when(first_change && !this.editors.compare.diff.limited, |el| {
                                    el.child(
                                        ui::button(
                                            ("editor-revert-block", index),
                                            "Restore block",
                                            false,
                                        )
                                        .text_size(px(10.))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.revert_editor_compare_block(
                                                    index,
                                                    expected.clone(),
                                                    generation,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    )
                                })
                                .when(first_change && !this.editors.compare.diff.limited, |el| {
                                    el.child(
                                        ui::button(
                                            ("editor-copy-block", index),
                                            "Copy block",
                                            false,
                                        )
                                        .text_size(px(10.))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.copy_editor_compare_block(
                                                    index,
                                                    copy_expected.clone(),
                                                    generation,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    )
                                }),
                        )
                    })
                    .collect::<Vec<_>>()
            })
        })
        .h(px(212.))
        .w_full();
        let dirty = self.active_document_dirty(cx);
        let summary = format!(
            "{} added · {} removed{}",
            comparison.diff.added,
            comparison.diff.removed,
            if comparison.diff.limited {
                " · display limited"
            } else {
                ""
            }
        );
        Some(
            div()
                .id("editor-compare-panel")
                .relative()
                .child(ui::layout_probe("editor-compare-panel"))
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .border_1()
                .border_color(rgb(palette().border))
                .rounded_md()
                .bg(rgb(palette().canvas))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().min_w_0().text_size(px(11.)).child(format!(
                            "Compare · {}",
                            comparison.label.as_deref().unwrap_or("choose a scope")
                        )))
                        .child(
                            ui::action(
                                "editor-compare-copy-review",
                                "Copy review",
                                None,
                                comparison.pending
                                    || comparison.error.is_some()
                                    || comparison.diff.limited
                                    || comparison.reference.is_none(),
                                cx.listener(|this, _: &(), _, cx| {
                                    let compare = &this.editors.compare;
                                    if compare.pending
                                        || compare.error.is_some()
                                        || compare.diff.limited
                                        || compare.reference.is_none()
                                    {
                                        return;
                                    }
                                    let mut review = format!(
                                        "Comparison: {}\n",
                                        compare.label.as_deref().unwrap_or("reference")
                                    );
                                    for line in &compare.diff.lines {
                                        let marker = match line.kind {
                                            CompareLineKind::Same => ' ',
                                            CompareLineKind::Added => '+',
                                            CompareLineKind::Removed => '-',
                                            CompareLineKind::Info => '#',
                                        };
                                        review.push(marker);
                                        review.push_str(&line.text);
                                        review.push('\n');
                                    }
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(review));
                                }),
                            )
                            .text_size(px(10.)),
                        )
                        .child(
                            ui::button(
                                "editor-compare-changes-only",
                                if changes_only {
                                    "Show all"
                                } else {
                                    "Changes only"
                                },
                                changes_only,
                            )
                            .text_size(px(10.))
                            .relative()
                            .child(ui::layout_probe("editor-compare-changes-only"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.editors.compare.changes_only =
                                    !this.editors.compare.changes_only;
                                cx.notify();
                            })),
                        )
                        .children(change_navigation_available.then(|| {
                            ui::button("editor-compare-change-prev", "Previous change", false)
                                .text_size(px(10.))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.select_editor_compare_change(true, cx)
                                }))
                        }))
                        .children(change_navigation_available.then(|| {
                            ui::button("editor-compare-change-next", "Next change", false)
                                .text_size(px(10.))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.select_editor_compare_change(false, cx)
                                }))
                        }))
                        .children(selected_change.map(|_| {
                            ui::button("editor-compare-copy-selected", "Copy selected block", false)
                                .text_size(px(10.))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.copy_selected_editor_compare_block(cx)
                                }))
                        }))
                        .children(selected_change.map(|_| {
                            ui::button(
                                "editor-compare-restore-selected",
                                "Restore selected block",
                                false,
                            )
                            .text_size(px(10.))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.restore_selected_editor_compare_block(cx)
                            }))
                        }))
                        .children(restore_all_available.then(|| {
                            ui::button(
                                "editor-compare-restore-all",
                                if restore_all_confirmed {
                                    "Confirm restore all"
                                } else {
                                    "Restore all"
                                },
                                restore_all_confirmed,
                            )
                            .text_size(px(10.))
                            .relative()
                            .child(ui::layout_probe("editor-compare-restore-all"))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.restore_all_editor_compare_changes(
                                        restore_all_generation,
                                        cx,
                                    );
                                },
                            ))
                        }))
                        .child(
                            ui::action(
                                "editor-compare-saved",
                                "Saved snapshot",
                                None,
                                comparison.mode == Some(CompareMode::Saved),
                                cx.listener(|this, _: &(), _, cx| {
                                    this.compare_editor_saved_buffer(cx)
                                }),
                            )
                            .text_size(px(10.)),
                        )
                        .child(
                            ui::action(
                                "editor-compare-disk",
                                "Disk now",
                                None,
                                comparison.mode == Some(CompareMode::Disk) && comparison.pending,
                                cx.listener(|this, _: &(), _, cx| this.compare_editor_disk(cx)),
                            )
                            .text_size(px(10.)),
                        )
                        .child(
                            ui::button("editor-compare-close", "Close", false)
                                .text_size(px(10.))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_editor_compare();
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(self.editors.compare_ref.clone()),
                        )
                        .child(
                            ui::action(
                                "editor-compare-ref-submit",
                                "Compare ref",
                                None,
                                comparison.mode == Some(CompareMode::GitRef) && comparison.pending,
                                cx.listener(|this, _: &(), _, cx| this.compare_editor_git_ref(cx)),
                            )
                            .text_size(px(10.)),
                        ),
                )
                .children(comparison.error.as_ref().map(|error| {
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette().error))
                        .child(error.clone())
                }))
                .children((comparison.pending).then(|| {
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette().muted))
                        .child("Reading comparison snapshot…")
                }))
                .children(
                    (!comparison.pending && comparison.error.is_none()).then(|| {
                        div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(10.))
                        .text_color(rgb(palette().muted))
                        .child(summary)
                        .children(selected_position.map(|position| {
                            div().child(format!(
                                "Selected change {position}/{}",
                                change_starts.len()
                            ))
                        }))
                        .child(if dirty {
                            "Current unsaved editor buffer is included; comparison is read-only."
                        } else {
                            "Current editor buffer is included; comparison is read-only."
                        })
                    }),
                )
                .child(
                    div()
                        .id("editor-compare-lines-scroll")
                        .min_h_0()
                        .min_w_0()
                        .overflow_x_scroll()
                        .child(list),
                )
                .into_any_element(),
        )
    }
    pub(super) fn editor_content(&self) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(self.editor.clone())
            .into_any_element()
    }
    pub(super) fn editor_preview(&self, cx: &App) -> gpui::AnyElement {
        div()
            .id("editor-markdown-preview")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .p_3()
            .child(ui::markdown::render(
                self.editor.read(cx).text(),
                "editor-preview",
            ))
            .into_any_element()
    }
    pub(super) fn editor_status(&self, cx: &App) -> gpui::AnyElement {
        let entry = self.editor.read(cx);
        let (line, column) = entry.cursor_position();
        let selection = entry.selected_text().chars().count();
        let newlines = if entry.text().contains("\r\n") {
            "CRLF"
        } else {
            "LF"
        };
        div()
            .flex()
            .items_center()
            .flex_wrap()
            .gap_3()
            .text_size(px(11.))
            .text_color(rgb(palette().muted))
            .flex_shrink_0()
            .border_t_1()
            .border_color(rgb(palette().border))
            .pt_1()
            .child(format!("Ln {line}, Col {column}"))
            .child(format!("{} bytes", entry.text().len()))
            .child(format!("UTF-8 · {newlines}"))
            .children((selection > 0).then(|| div().child(format!("{selection} selected"))))
            .into_any_element()
    }
}

#[cfg(test)]
mod conflict_confirmation_tests {
    use super::{
        CompareLineKind, ReloadConfirmation, compare_change_starts, compare_text,
        compare_visible_indices, next_compare_change, reload_confirmation_matches,
        restore_compare_all, revert_compare_block, three_way_merge,
    };

    #[test]
    fn restores_only_one_comparison_block_and_refuses_stale_or_crlf_input() {
        let reference = "first\nkeep\nsecond\n";
        let current = "changed\nkeep\nchanged again\n";
        let diff = compare_text(reference, current);
        let first = diff
            .lines
            .iter()
            .position(|line| line.kind == CompareLineKind::Removed)
            .unwrap();
        assert_eq!(
            revert_compare_block(reference, current, &diff, first).as_deref(),
            Some("first\nkeep\nchanged again\n")
        );
        assert!(
            revert_compare_block(
                reference,
                "edited again\nkeep\nchanged again\n",
                &diff,
                first
            )
            .is_none()
        );
        assert!(
            revert_compare_block("first\r\nkeep\r\n", "changed\r\nkeep\r\n", &diff, first)
                .is_none()
        );
    }

    #[test]
    fn restore_all_comparison_changes_requires_the_exact_current_diff() {
        let reference = "first\r\nkeep\r\n";
        let current = "changed\r\nkeep\r\n";
        let diff = compare_text(reference, current);

        assert_eq!(
            restore_compare_all(reference, current, &diff).as_deref(),
            Some(reference)
        );
        assert!(restore_compare_all(reference, "newer\r\nkeep\r\n", &diff).is_none());

        let mut limited = diff.clone();
        limited.limited = true;
        assert!(restore_compare_all(reference, current, &limited).is_none());

        let identical = compare_text(reference, reference);
        assert!(restore_compare_all(reference, reference, &identical).is_none());
    }

    #[test]
    fn changes_only_filter_preserves_original_diff_indices_and_info_rows() {
        let diff = compare_text("same\nbefore\nold\nafter\n", "same\nbefore\nnew\nafter");
        let visible = compare_visible_indices(&diff, true);
        assert_eq!(
            visible,
            diff.lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| {
                    (line.kind != CompareLineKind::Same).then_some(index)
                })
                .collect::<Vec<_>>()
        );
        assert!(visible.iter().any(|index| {
            diff.lines[*index].kind == CompareLineKind::Removed && diff.lines[*index].text == "old"
        }));
        assert!(visible.iter().any(|index| {
            diff.lines[*index].kind == CompareLineKind::Added && diff.lines[*index].text == "new"
        }));
        assert!(
            visible
                .iter()
                .any(|index| diff.lines[*index].kind == CompareLineKind::Info)
        );
        assert_eq!(
            compare_visible_indices(&diff, false),
            (0..diff.lines.len()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn comparison_change_cursor_visits_only_block_starts_and_wraps() {
        let diff = compare_text(
            "one\nkeep\ntwo\nkeep2\nthree\n",
            "ONE\nkeep\nTWO\nkeep2\nTHREE\n",
        );
        let starts = compare_change_starts(&diff);
        assert_eq!(starts.len(), 3);
        assert_eq!(next_compare_change(&diff, None, false), Some(starts[0]));
        assert_eq!(
            next_compare_change(&diff, Some(starts[0]), false),
            Some(starts[1])
        );
        assert_eq!(
            next_compare_change(&diff, Some(starts[0]), true),
            Some(starts[2])
        );
        assert_eq!(
            next_compare_change(&diff, Some(starts[2]), false),
            Some(starts[0])
        );

        let unchanged = compare_text("same\n", "same\n");
        assert!(compare_change_starts(&unchanged).is_empty());
        assert_eq!(next_compare_change(&unchanged, None, false), None);
    }

    #[test]
    fn comparison_marks_replacements_and_keeps_context_lines() {
        let diff = compare_text("one\nbefore\nold\nafter\n", "one\nbefore\nnew\nafter\n");

        assert_eq!(diff.removed, 1);
        assert_eq!(diff.added, 1);
        assert!(diff.lines.iter().any(|line| {
            line.kind == CompareLineKind::Removed && line.old == Some(3) && line.text == "old"
        }));
        assert!(diff.lines.iter().any(|line| {
            line.kind == CompareLineKind::Added && line.new == Some(3) && line.text == "new"
        }));
        assert!(
            diff.lines
                .iter()
                .any(|line| { line.kind == CompareLineKind::Same && line.text == "after" })
        );
    }

    #[test]
    fn comparison_reports_newline_metadata_without_content_noise() {
        let diff = compare_text("same\r\n", "same");

        assert_eq!(diff.added, 0);
        assert_eq!(diff.removed, 0);
        assert_eq!(diff.lines[0].kind, CompareLineKind::Same);
        assert!(diff.lines.iter().any(|line| {
            line.kind == CompareLineKind::Info && line.text.contains("Line endings differ")
        }));
        assert!(diff.lines.iter().any(|line| {
            line.kind == CompareLineKind::Info && line.text.contains("Final newline differs")
        }));
    }

    #[test]
    fn comparison_stops_before_building_an_unbounded_lcs_table() {
        let old = vec!["old"; 1500].join("\n");
        let new = vec!["new"; 1500].join("\n");

        let diff = compare_text(&old, &new);
        assert!(diff.limited);
        assert!(diff.added == 0 && diff.removed == 0);
    }

    #[test]
    fn three_way_merge_combines_disjoint_edits_and_keeps_line_endings() {
        let merge = three_way_merge(
            "one\r\ntwo\r\nthree\r\n",
            "ONE\r\ntwo\r\nthree\r\n",
            "one\r\ntwo\r\nTHREE\r\n",
        )
        .unwrap();

        assert_eq!(merge.text.as_deref(), Some("ONE\r\ntwo\r\nTHREE\r\n"));
        assert_eq!(merge.local_edits, 1);
        assert_eq!(merge.disk_edits, 1);
        assert_eq!(merge.conflicts, 0);
    }

    #[test]
    fn three_way_merge_keeps_the_buffer_when_edits_overlap() {
        let merge = three_way_merge(
            "before\ntarget\nafter\n",
            "before\nlocal\nafter\n",
            "before\ndisk\nafter\n",
        )
        .unwrap();

        assert_eq!(merge.text, None);
        assert!(merge.conflicts > 0);
    }

    #[test]
    fn three_way_merge_deduplicates_identical_edits() {
        let merge = three_way_merge("old\n", "new\n", "new\n").unwrap();

        assert_eq!(merge.text.as_deref(), Some("new\n"));
        assert_eq!(merge.conflicts, 0);
    }

    #[test]
    fn reload_confirmation_is_bound_to_tab_version_and_exact_buffer() {
        let version = synara_runtime::FileVersion("disk-v1".into());
        let confirmation = ReloadConfirmation {
            tab_id: 7,
            disk_version: version.clone(),
            buffer: "unsaved edit".into(),
        };

        assert!(reload_confirmation_matches(
            &confirmation,
            7,
            &version,
            "unsaved edit",
            false,
        ));
        assert!(!reload_confirmation_matches(
            &confirmation,
            8,
            &version,
            "unsaved edit",
            false,
        ));
        assert!(!reload_confirmation_matches(
            &confirmation,
            7,
            &synara_runtime::FileVersion("disk-v2".into()),
            "unsaved edit",
            false,
        ));
        assert!(!reload_confirmation_matches(
            &confirmation,
            7,
            &version,
            "newer unsaved edit",
            false,
        ));
        assert!(!reload_confirmation_matches(
            &confirmation,
            7,
            &version,
            "unsaved edit",
            true,
        ));
    }
}
