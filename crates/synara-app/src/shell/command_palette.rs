//! Searchable native launcher. Commands dispatch to existing owners, not a shell.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;

pub(super) struct PaletteState {
    pub open: bool,
    query: Entity<TextEntry>,
    selected: usize,
    previous: Option<FocusHandle>,
    scroll: gpui::ScrollHandle,
    _subscription: Subscription,
}
impl PaletteState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| {
            TextEntry::new(
                "Search commands, threads, projects or open files",
                EntryMode::SingleLine,
                40.,
                cx,
            )
        });
        let subscription = cx.subscribe(&query, |this, _, event, cx| {
            if matches!(event, EntryEvent::Changed) {
                this.command_palette.selected = 0;
                this.command_palette
                    .scroll
                    .set_offset(gpui::point(px(0.), px(0.)));
            }
            cx.notify();
        });
        Self {
            open: false,
            query,
            selected: 0,
            previous: None,
            scroll: gpui::ScrollHandle::new(),
            _subscription: subscription,
        }
    }
}
#[derive(Clone)]
enum Action {
    DebugWorkflow,
    Goals,
    Recap,
    Handoff,
    ToggleZen,
    Appearance,
    Attention,
    Panel(Panel),
    NewChat,
    NewHub,
    Outputs,
    Sidebar,
    Environment,
    ThreadSearch,
    MessageSearch,
    Notes,
    OpenProject,
    FileNameSearch,
    SourceSearch,
    Find,
    Replace,
    GoToLine,
    ToggleTree,
    Task(TaskId),
    Project(ProjectId),
    File(PathBuf),
}
struct Command {
    title: String,
    detail: String,
    glyph: Glyph,
    action: Action,
}
impl Shell {
    pub(super) fn open_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.hubs.pending(cx)
            || self.close != CloseState::Open
            || self.explorer.modal_open()
            || self.kanban.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.saved_context.dialog.is_some()
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
        {
            return;
        }
        self.command_palette.previous = window.focused(cx);
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        self.command_palette
            .query
            .update(cx, |input, cx| input.clear(cx));
        self.command_palette.selected = 0;
        self.command_palette
            .scroll
            .set_offset(gpui::point(px(0.), px(0.)));
        self.command_palette.open = true;
        self.focus_composer = false;
        window.focus(&self.command_palette.query.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn dismiss_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.command_palette.open = false;
        if let Some(previous) = self.command_palette.previous.take() {
            window.focus(&previous, cx);
        }
        cx.notify();
    }
    fn palette_commands(&self, cx: &App) -> Vec<Command> {
        let query = self
            .command_palette
            .query
            .read(cx)
            .text()
            .trim()
            .to_lowercase();
        let (kind, query) = match query.chars().next() {
            Some('>') => (1, query[1..].trim()),
            Some('@') => (2, query[1..].trim()),
            Some('/') => (3, query[1..].trim()),
            _ => (0, query.as_str()),
        };
        let mut commands = Vec::new();
        let mut add = |category: u8, title: String, detail: String, glyph, action| {
            if kind != 0 && kind != category {
                return;
            }
            let haystack = format!("{title} {detail}").to_lowercase();
            if query.split_whitespace().all(|word| haystack.contains(word)) {
                commands.push(Command {
                    title,
                    detail,
                    glyph,
                    action,
                });
            }
        };
        for (title, detail, glyph, action) in [
            (
                "Toggle Zen mode",
                "Focus view · Ctrl/Cmd+Alt+Z",
                Glyph::Goal,
                Action::ToggleZen,
            ),
            (
                "Customize appearance",
                "Themes, wallpaper, glass and motion",
                Glyph::Palette,
                Action::Appearance,
            ),
            (
                "Active tasks and decisions",
                "Running chats and pending requests",
                Glyph::Bell,
                Action::Attention,
            ),
            (
                "Hubs",
                "Shared instructions, knowledge and Library",
                Glyph::Folders,
                Action::Panel(Panel::Hubs),
            ),
            (
                "New thread",
                "Create a standalone chat",
                Glyph::Compose,
                Action::NewChat,
            ),
            (
                "Chat",
                "Ctrl/Cmd+1",
                Glyph::Chat,
                Action::Panel(Panel::Conversation),
            ),
            (
                "Explorer",
                "Files · Ctrl/Cmd+2",
                Glyph::Files,
                Action::Panel(Panel::Files),
            ),
            (
                "Changes",
                "Git review · Ctrl/Cmd+3",
                Glyph::Changes,
                Action::Panel(Panel::Changes),
            ),
            (
                "Terminal",
                "Open panel without starting a shell · Ctrl/Cmd+4",
                Glyph::Terminal,
                Action::Panel(Panel::Terminal),
            ),
            (
                "Browser",
                "Tabs, navigation and isolated agent browser use",
                Glyph::Browser,
                Action::Panel(Panel::Browser),
            ),
            (
                "Automations",
                "Durable schedules and owned run history",
                Glyph::Clock,
                Action::Panel(Panel::Automations),
            ),
            (
                "Device",
                "Real device discovery and capture",
                Glyph::Window,
                Action::Panel(Panel::Device),
            ),
            (
                "Side chats",
                "Independent related conversations",
                Glyph::Chat,
                Action::Panel(Panel::SideChats),
            ),
            (
                "Kanban",
                "Projects and tasks · Ctrl/Cmd+9",
                Glyph::Kanban,
                Action::Panel(Panel::Kanban),
            ),
            (
                "Settings",
                "Preferences · Ctrl/Cmd+6",
                Glyph::Settings,
                Action::Panel(Panel::Settings),
            ),
            (
                "Agent registry",
                "Installed and available agents · Ctrl/Cmd+7",
                Glyph::Agent,
                Action::Panel(Panel::Registry),
            ),
            (
                "Remote workspace",
                "Pinned SSH connection · Ctrl/Cmd+8",
                Glyph::Browser,
                Action::Panel(Panel::Remote),
            ),
            (
                "Protocol inspector",
                "ACP session details · Ctrl/Cmd+5",
                Glyph::Debug,
                Action::Panel(Panel::Inspector),
            ),
            (
                "Find thread",
                "Search all chats · Ctrl/Cmd+K",
                Glyph::Search,
                Action::ThreadSearch,
            ),
            (
                "Open project",
                "Choose a workspace directory",
                Glyph::Folder,
                Action::OpenProject,
            ),
            (
                "Find file",
                "Search project file names · Ctrl/Cmd+P",
                Glyph::Files,
                Action::FileNameSearch,
            ),
            (
                "Search source",
                "Find matching lines · Ctrl/Cmd+Shift+F",
                Glyph::Search,
                Action::SourceSearch,
            ),
            (
                "Toggle sidebar",
                "Show or hide navigation",
                Glyph::Panel,
                Action::Sidebar,
            ),
            (
                "Toggle Environment",
                "Show or hide the workspace panel",
                Glyph::Window,
                Action::Environment,
            ),
            (
                "What's New and Releases",
                "Native build version, local history and update configuration",
                Glyph::Help,
                Action::Panel(Panel::Help),
            ),
            (
                "Help and shortcuts",
                "Keyboard reference and licenses",
                Glyph::Help,
                Action::Panel(Panel::Help),
            ),
        ] {
            let detail = if let Action::Panel(panel) = &action {
                self.panel_shortcut_label(*panel).map_or_else(
                    || detail.to_owned(),
                    |key| format!("{key} | Native navigation"),
                )
            } else {
                detail.to_owned()
            };
            add(1, title.into(), detail, glyph, action);
        }
        if self.settings.value.general.show_studio {
            add(
                1,
                "New Hub".into(),
                "Optional shared work context".into(),
                Glyph::Blocks,
                Action::NewHub,
            );
        }
        if self.selected.is_some() {
            for (title, detail, glyph, action) in [
                (
                    "Search this conversation",
                    "Find messages and work details",
                    Glyph::Search,
                    Action::MessageSearch,
                ),
                (
                    "Thread recap",
                    "Generate, review and cache a bounded thread recap",
                    Glyph::Notebook,
                    Action::Recap,
                ),
                (
                    "Continue with another agent or model",
                    "Electron-style handoff menu with immediate unsent creation",
                    Glyph::Handoff,
                    Action::Handoff,
                ),
                (
                    "Persistent thread goal",
                    "Explicit bounded pursuit, pause, blockers and achievements",
                    Glyph::Goal,
                    Action::Goals,
                ),
                (
                    "Debug workflow",
                    "Evidence-first Observe, Reproduce, Investigate, Fix, Verify",
                    Glyph::Debug,
                    Action::DebugWorkflow,
                ),
                (
                    "Chat notes and checklist",
                    "User-owned saved context",
                    Glyph::Notebook,
                    Action::Notes,
                ),
                (
                    "Hub Library",
                    "Workspace files and attributed outputs",
                    Glyph::Blocks,
                    Action::Outputs,
                ),
            ] {
                add(1, title.into(), detail.into(), glyph, action);
            }
        }
        if self.document.is_some() {
            for (title, detail, glyph, action) in [
                (
                    "Find in file",
                    "Exact text · Ctrl/Cmd+F",
                    Glyph::Search,
                    Action::Find,
                ),
                (
                    "Replace in file",
                    "Undoable buffer edits · Ctrl/Cmd+H",
                    Glyph::Compose,
                    Action::Replace,
                ),
                (
                    "Go to line",
                    "Line:column · Ctrl/Cmd+G",
                    Glyph::Down,
                    Action::GoToLine,
                ),
                (
                    "Toggle file tree",
                    "Give the editor more room",
                    Glyph::Folders,
                    Action::ToggleTree,
                ),
            ] {
                add(1, title.into(), detail.into(), glyph, action);
            }
        }
        let mut tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| task.state != TaskState::Archived)
            .collect();
        tasks.sort_by_key(|task| std::cmp::Reverse(task.updated_at_ms));
        for task in tasks {
            add(
                2,
                task.title.clone(),
                format!("Thread · {:?}", task.scope),
                self.agent_glyph(&task.agent_id),
                Action::Task(task.id),
            );
        }
        for project in self
            .catalog
            .projects
            .iter()
            .filter(|project| !self.is_chat_workspace(project))
        {
            add(
                3,
                project.name.clone(),
                "Project".into(),
                Glyph::Folder,
                Action::Project(project.id),
            );
        }
        for path in self.open_editor_paths() {
            add(
                4,
                path.display().to_string(),
                "Open file".into(),
                Glyph::Files,
                Action::File(path),
            );
        }
        commands.truncate(20);
        commands
    }
    fn execute_palette_command(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_command_palette(window, cx);
        match action {
            Action::Recap => self.open_recap(cx),
            Action::Goals => self.open_goals(cx),
            Action::DebugWorkflow => self.open_debug(cx),
            Action::Handoff => self.open_handoff_menu(window, cx),
            Action::ToggleZen => self.toggle_zen(cx),
            Action::Appearance => self.open_appearance(cx),
            Action::Attention => self.open_attention(window, cx),
            Action::Panel(panel) => {
                if panel == Panel::Conversation {
                    self.show_conversation(cx);
                } else if panel == Panel::Hubs {
                    self.show_hubs(cx);
                } else {
                    self.set_panel(panel, cx);
                }
            }
            Action::NewChat => {
                self.task_title.update(cx, |input, cx| input.clear(cx));
                self.create_chat(TaskScope::Chat, cx);
            }
            Action::NewHub => self.edit_hub(true, cx),
            Action::Sidebar => {
                if self.zen_active() {
                    self.settings.personalization.navigation_shown =
                        !self.settings.personalization.navigation_shown;
                    self.focus_composer = !self.settings.personalization.navigation_shown;
                } else {
                    self.toggle_sidebar(cx);
                }
            }
            Action::Environment => {
                if self.zen_active() {
                    self.toggle_zen_tools(cx);
                } else if self.dock_open() {
                    self.hide_environment(cx);
                } else {
                    self.set_panel(Panel::Dock, cx);
                }
            }
            Action::ThreadSearch => self.open_thread_finder(window, cx),
            Action::MessageSearch => {
                self.show_conversation(cx);
                self.open_message_search(window, cx);
            }
            Action::Notes => self.open_saved_context(window, cx),
            Action::Outputs => self.open_studio_outputs(cx),
            Action::OpenProject => self.browse_workspace(cx),
            Action::FileNameSearch => self.open_file_name_search(window, cx),
            Action::SourceSearch => self.open_content_search(window, cx),
            Action::Task(id) => {
                if self.select_task(id, cx) {
                    self.show_conversation(cx);
                }
            }
            Action::Project(id) => self.navigate_project(id, cx),
            Action::File(path) => {
                self.set_panel(Panel::Files, cx);
                self.open_file(path, cx);
            }
            Action::Find | Action::Replace => {
                self.set_panel(Panel::Files, cx);
                self.open_editor_find(matches!(action, Action::Replace), window, cx);
            }
            Action::GoToLine => {
                self.set_panel(Panel::Files, cx);
                self.open_editor_goto(window, cx);
            }
            Action::ToggleTree => {
                self.set_panel(Panel::Files, cx);
                self.editors.tree_visible = !self.editors.tree_visible;
            }
        }
        cx.notify();
    }
    pub(super) fn command_palette_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.prefer_character_input
            || event.is_held
            || self.command_palette.query.read(cx).is_composing()
        {
            return false;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if (modifiers.control || modifiers.platform)
            && modifiers.shift
            && !modifiers.alt
            && key == "p"
        {
            if self.command_palette.open {
                self.dismiss_command_palette(window, cx);
            } else {
                self.open_command_palette(window, cx);
            }
            return true;
        }
        if !self.command_palette.open {
            return false;
        }
        match key {
            "escape" => self.dismiss_command_palette(window, cx),
            "up" | "down" => {
                self.command_palette
                    .scroll
                    .set_offset(gpui::point(px(0.), px(0.)));
                let count = self.palette_commands(cx).len();
                if count > 0 {
                    self.command_palette.selected = if key == "up" {
                        (self.command_palette.selected + count - 1) % count
                    } else {
                        (self.command_palette.selected + 1) % count
                    };
                }
                cx.notify();
            }
            "tab" => {
                window.focus(&self.command_palette.query.read(cx).focus_handle(cx), cx);
            }
            "enter" => {
                let commands = self.palette_commands(cx);
                if let Some(command) = commands.get(
                    self.command_palette
                        .selected
                        .min(commands.len().saturating_sub(1)),
                ) {
                    self.execute_palette_command(command.action.clone(), window, cx);
                }
            }
            _ => return false,
        }
        true
    }
    pub(super) fn command_palette_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let commands = self.palette_commands(cx);
        let first = self.command_palette.selected.saturating_sub(7);
        div().absolute().inset_0().occlude().bg(gpui::rgba(0x00000066)).flex().justify_center().pt(px(76.)).px_4()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| { this.dismiss_command_palette(window, cx); cx.stop_propagation(); }))
            .child(div().id("command-palette-dialog").role(gpui::Role::Dialog).aria_label("Command palette")
                .occlude().tab_group().w(px(620.)).max_w_full().h(px(500.)).rounded_xl().bg(rgb(palette().overlay))
                .border_1().border_color(rgb(palette().border)).flex().flex_col().overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().p_3().child(self.command_palette.query.clone()))
                .child(div().id("command-palette-results").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.command_palette.scroll).px_2()
                    .children(commands.iter().enumerate().skip(first).map(|(index, command)| {
                        let action = command.action.clone();
                        ui::action(("command", index), command.title.clone(), Some(command.glyph), index == self.command_palette.selected,
                            cx.listener(move |this, _: &(), window, cx| this.execute_palette_command(action.clone(), window, cx)))
                            .w_full().h(px(38.)).min_w_0().child(div().flex_1())
                            .child(div().max_w(px(230.)).text_ellipsis().text_size(px(11.)).text_color(rgb(palette().muted)).child(command.detail.clone()))
                    }))
                    .children(commands.is_empty().then(|| div().p_4().text_color(rgb(palette().muted)).child("No matching commands, threads, projects or open files."))))
                .child(div().px_3().py_2().text_size(px(11.)).text_color(rgb(palette().muted))
                    .child("↑ ↓ Navigate    Enter Run    Esc Close    > Commands    @ Threads    / Projects")))
            .into_any_element()
    }
}
