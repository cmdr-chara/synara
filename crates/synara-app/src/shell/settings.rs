//! Native settings pages. Editable controls persist through WorkspaceService;
//! sections without a native service say so instead of displaying invented data.
use super::*;
mod chat;
use crate::ui::menu::{Choice, ChoiceEvent, ChoiceMenu};
use crate::ui::{self, Glyph, palette};
use gpui::{FocusHandle, Pixels, Point};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    General,
    Profile,
    Appearance,
    Notifications,
    Behavior,
    Keybindings,
    Usage,
    AppSnap,
    Mcp,
    Providers,
    Models,
    Skills,
    Worktrees,
    System,
    Archived,
}
struct SectionInfo {
    section: Section,
    id: &'static str,
    group: &'static str,
    label: &'static str,
    icon: Glyph,
    description: &'static str,
}
const SECTIONS: &[SectionInfo] = &[
    SectionInfo {
        section: Section::General,
        id: "general",
        group: "Personal",
        label: "General",
        icon: Glyph::Settings,
        description: "Choose defaults for new chats, navigation, and the Environment panel.",
    },
    SectionInfo {
        section: Section::Profile,
        id: "profile",
        group: "Personal",
        label: "Profile",
        icon: Glyph::User,
        description: "Your profile and activity in this installation of Synara.",
    },
    SectionInfo {
        section: Section::Appearance,
        id: "appearance",
        group: "Personal",
        label: "Appearance",
        icon: Glyph::Palette,
        description: "Customize the theme, typography, and motion.",
    },
    SectionInfo {
        section: Section::Notifications,
        id: "notifications",
        group: "Personal",
        label: "Notifications",
        icon: Glyph::Bell,
        description: "Updates when work finishes or needs attention.",
    },
    SectionInfo {
        section: Section::Behavior,
        id: "behavior",
        group: "Personal",
        label: "Chat behavior",
        icon: Glyph::Sliders,
        description: "Live responses, follow-ups, and permission requests.",
    },
    SectionInfo {
        section: Section::Keybindings,
        id: "keybindings",
        group: "Personal",
        label: "Keybindings",
        icon: Glyph::Shortcut,
        description: "Keyboard shortcuts for Synara.",
    },
    SectionInfo {
        section: Section::Usage,
        id: "usage",
        group: "Personal",
        label: "Usage & limits",
        icon: Glyph::Gauge,
        description: "Usage reported by the agent for your current chat.",
    },
    SectionInfo {
        section: Section::AppSnap,
        id: "appsnap",
        group: "Integrations",
        label: "AppSnap",
        icon: Glyph::Capture,
        description: "Capture an app window into a chat.",
    },
    SectionInfo {
        section: Section::Mcp,
        id: "mcp",
        group: "Integrations",
        label: "MCP connections",
        icon: Glyph::Plugin,
        description: "Connections to external tools and services.",
    },
    SectionInfo {
        section: Section::Providers,
        id: "providers",
        group: "Coding",
        label: "Agent providers",
        icon: Glyph::Puzzle,
        description: "Choose coding agents and manage their installed tools.",
    },
    SectionInfo {
        section: Section::Models,
        id: "models",
        group: "Coding",
        label: "Models & writing",
        icon: Glyph::Brain,
        description: "Models available to your current chat.",
    },
    SectionInfo {
        section: Section::Skills,
        id: "skills",
        group: "Coding",
        label: "Agent skills",
        icon: Glyph::Blocks,
        description: "Reusable workflows from your agents.",
    },
    SectionInfo {
        section: Section::Worktrees,
        id: "worktrees",
        group: "Coding",
        label: "Managed worktrees",
        icon: Glyph::BranchSimple,
        description: "Isolated workspaces created by Synara.",
    },
    SectionInfo {
        section: Section::System,
        id: "system",
        group: "System",
        label: "System tools",
        icon: Glyph::Toolbox,
        description: "Sessions, remote connections, and recovery tools.",
    },
    SectionInfo {
        section: Section::Archived,
        id: "archived",
        group: "Archived",
        label: "Archived threads",
        icon: Glyph::Archive,
        description: "Find and restore threads you previously archived.",
    },
];
#[derive(Clone, Copy)]
enum ChoiceKind {
    Provider,
    Projects,
    Threads,
    DarkTheme,
}
pub(super) struct SettingsPopup {
    view: Entity<ChoiceMenu>,
    position: Point<Pixels>,
    return_focus: FocusHandle,
    _subscription: Subscription,
}
pub(super) struct SettingsState {
    pub value: AppSettings,
    pub saving: bool,
    pub popup: Option<SettingsPopup>,
    scroll: gpui::ScrollHandle,
    section: Section,
    search: Entity<TextEntry>,
    name: Entity<TextEntry>,
    username: Entity<TextEntry>,
    ui_font: Entity<TextEntry>,
    code_font: Entity<TextEntry>,
    pub activity: Option<ProfileActivity>,
    pub activity_loading: bool,
    _subscriptions: Vec<Subscription>,
}
#[derive(Default)]
pub(super) struct ProfileActivity {
    prompts: usize,
    threads: usize,
    tokens: Option<u64>,
    days: BTreeMap<i64, usize>,
    error: Option<String>,
}
impl SettingsState {
    pub fn new(value: AppSettings, cx: &mut Context<Shell>) -> Self {
        let search = cx.new(|cx| {
            TextEntry::new("Search settings...", EntryMode::SingleLine, 28., cx)
                .with_leading_icon(Glyph::Search)
        });
        let name = cx.new(|cx| TextEntry::new("Display name", EntryMode::SingleLine, 32., cx));
        let username = cx.new(|cx| TextEntry::new("Username", EntryMode::SingleLine, 32., cx));
        let ui_font = cx.new(|cx| TextEntry::new("System default", EntryMode::SingleLine, 32., cx));
        let code_font =
            cx.new(|cx| TextEntry::new("System default", EntryMode::SingleLine, 32., cx));
        for (entry, text) in [
            (&name, value.profile.name.clone()),
            (&username, value.profile.username.clone()),
            (
                &ui_font,
                value.appearance.fonts.ui_family.clone().unwrap_or_default(),
            ),
            (
                &code_font,
                value
                    .appearance
                    .fonts
                    .code_family
                    .clone()
                    .unwrap_or_default(),
            ),
        ] {
            entry.update(cx, |entry, cx| entry.set_text(text, cx));
        }
        let subscriptions = vec![cx.subscribe(&search, |this, _, event, cx| {
            if matches!(event, EntryEvent::Submit) {
                let query = this.settings.search.read(cx).text().trim().to_lowercase();
                if let Some(info) = SECTIONS.iter().find(|info| settings_match(info, &query)) {
                    this.open_settings_section(info.section, cx);
                }
            }
            cx.notify();
        })];
        Self {
            value,
            saving: false,
            popup: None,
            scroll: gpui::ScrollHandle::new(),
            section: Section::General,
            search,
            name,
            username,
            ui_font,
            code_font,
            activity: None,
            activity_loading: false,
            _subscriptions: subscriptions,
        }
    }
}
fn settings_match(info: &SectionInfo, query: &str) -> bool {
    info.label.to_lowercase().contains(query) || info.description.to_lowercase().contains(query)
}
fn card() -> gpui::Div {
    div()
        .w_full()
        .rounded_2xl()
        .border_1()
        .border_color(rgb(palette().border))
        .overflow_hidden()
        .flex()
        .flex_col()
}
fn heading(label: &'static str) -> gpui::Div {
    div()
        .mt_7()
        .mb_3()
        .px_2()
        .text_color(rgb(palette().muted))
        .child(label)
}
fn row(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> gpui::Div {
    let description = description.into();
    div()
        .px_3()
        .py_3()
        .border_b_1()
        .border_color(rgb(palette().border))
        .flex()
        .items_center()
        .gap_5()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(title.into())
                .children((!description.is_empty()).then(|| {
                    div()
                        .text_color(rgb(palette().muted))
                        .line_height(px(22.))
                        .child(description)
                })),
        )
        .child(div().flex_shrink_0().max_w(px(224.)).child(control))
}
fn empty(title: &'static str, detail: &'static str) -> gpui::Div {
    card().p_6().gap_2().child(title).child(
        div()
            .text_color(rgb(palette().muted))
            .line_height(px(22.))
            .child(detail),
    )
}
impl Shell {
    fn open_settings_section(&mut self, section: Section, cx: &mut Context<Self>) {
        self.settings.section = section;
        self.settings.popup = None;
        self.settings.scroll.set_offset(gpui::point(px(0.), px(0.)));
        self.settings.search.update(cx, |entry, cx| entry.clear(cx));
        cx.notify();
    }
    fn save_setting(&mut self, change: impl FnOnce(&mut AppSettings), cx: &mut Context<Self>) {
        if self.settings.saving {
            return;
        }
        let mut value = self.settings.value.clone();
        change(&mut value);
        if let Err(error) = value.validate() {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.settings.saving = true;
        self.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let error = workspace
                .save_settings(value.clone())
                .await
                .err()
                .map(|error| error.to_string());
            Ok(Update::SettingsSaved(Box::new(value), error))
        });
        cx.notify();
    }
    pub(super) fn load_profile_activity(&mut self) {
        if self.settings.activity_loading {
            return;
        }
        self.settings.activity_loading = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let mut activity = ProfileActivity::default();
            let result = async {
                for task in workspace.catalog().await?.tasks {
                    let thread = workspace.thread(task.thread_id).await?;
                    if !thread.turns.is_empty() {
                        activity.threads += 1;
                    }
                    activity.prompts += thread.turns.len();
                    for turn in thread.turns {
                        *activity
                            .days
                            .entry(turn.started_at_ms.div_euclid(86_400_000))
                            .or_default() += 1;
                    }
                    if let (Some(input), Some(output)) =
                        (thread.usage.input_tokens, thread.usage.output_tokens)
                    {
                        activity.tokens = Some(
                            activity
                                .tokens
                                .unwrap_or(0)
                                .saturating_add(input)
                                .saturating_add(output),
                        );
                    }
                }
                Ok::<_, WorkspaceError>(())
            }
            .await;
            activity.error = result.err().map(|error| error.to_string());
            Ok(Update::ProfileActivity(activity))
        });
    }
    pub(super) fn settings_sidebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.settings.search.read(cx).text().trim().to_lowercase();
        div()
            .id("settings-sidebar")
            .w(px(ui::SIDEBAR_WIDTH))
            .h_full()
            .bg(rgb(palette().sidebar))
            .flex()
            .flex_col()
            .min_h_0()
            .child(
                div().p_2().pb_2().child(
                    ui::action(
                        "settings-back",
                        "Back to app",
                        Some(Glyph::Back),
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.settings.popup = None;
                            this.set_panel(Panel::Conversation, cx);
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("settings-back")),
                ),
            )
            .child(
                div().px_3().pb_1().child(
                    div()
                        .relative()
                        .child(self.settings.search.clone())
                        .child(ui::layout_probe("settings-search")),
                ),
            )
            .child(
                div()
                    .id("settings-sections")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_4()
                    .children(
                        ["Personal", "Integrations", "Coding", "System", "Archived"]
                            .into_iter()
                            .filter_map(|group| {
                                let entries: Vec<_> = SECTIONS
                                    .iter()
                                    .filter(|info| {
                                        info.group == group && settings_match(info, &query)
                                    })
                                    .collect();
                                if entries.is_empty() {
                                    return None;
                                }
                                Some(
                                    div()
                                        .mt_3()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .px_2()
                                                .pb_2()
                                                .text_color(rgb(palette().muted))
                                                .child(group),
                                        )
                                        .children(entries.into_iter().map(|info| {
                                            let section = info.section;
                                            ui::action(
                                                info.id,
                                                info.label,
                                                Some(info.icon),
                                                self.settings.section == section,
                                                cx.listener(move |this, _: &(), _, cx| {
                                                    this.open_settings_section(section, cx);
                                                }),
                                            )
                                            .h(px(30.))
                                            .relative()
                                            .child(ui::layout_probe(info.id))
                                        })),
                                )
                            }),
                    )
                    .children(
                        (!query.is_empty()
                            && !SECTIONS.iter().any(|info| settings_match(info, &query)))
                        .then(|| {
                            div()
                                .p_3()
                                .text_color(rgb(palette().muted))
                                .child("No matching settings.")
                        }),
                    ),
            )
            .child(
                div()
                    .h(px(36.))
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .px_2()
                    .child(
                        ui::chrome_button(
                            "settings-help",
                            "Help and licenses",
                            Glyph::Help,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Help, cx)),
                        )
                        .opacity(0.5),
                    ),
            )
            .into_any_element()
    }
    fn choice_button(
        &self,
        id: &'static str,
        label: String,
        kind: ChoiceKind,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        ui::button_shell(id, label.clone(), false)
            .w(px(176.))
            .min_h(px(32.))
            .border_1()
            .border_color(rgb(palette().border))
            .rounded_lg()
            .bg(gpui::rgba(0))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().text_ellipsis().child(label))
            .child(ui::icon(Glyph::Chevron).size(px(11.)))
            .relative()
            .child(ui::layout_probe(id))
            .when(self.settings.saving, |button| button.opacity(0.5))
            .on_click(
                cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                    if this.settings.saving {
                        return;
                    }
                    let options: Vec<(String, bool)> = match kind {
                        ChoiceKind::Provider => this
                            .profiles
                            .iter()
                            .map(|profile| {
                                (
                                    profile.name.clone(),
                                    this.settings.value.general.default_provider.as_ref()
                                        == Some(&profile.id),
                                )
                            })
                            .collect(),
                        ChoiceKind::Projects => vec![
                            (
                                "Manual order".into(),
                                !this.settings.value.general.alphabetical_projects,
                            ),
                            (
                                "Alphabetical".into(),
                                this.settings.value.general.alphabetical_projects,
                            ),
                        ],
                        ChoiceKind::Threads => vec![
                            (
                                "Recently active".into(),
                                !this.settings.value.general.oldest_threads_first,
                            ),
                            (
                                "Oldest first".into(),
                                this.settings.value.general.oldest_threads_first,
                            ),
                        ],
                        ChoiceKind::DarkTheme => vec![
                            (
                                "Synara".into(),
                                this.settings.value.appearance.dark_theme
                                    == DarkThemePreference::Synara,
                            ),
                            (
                                "Dracula".into(),
                                this.settings.value.appearance.dark_theme
                                    == DarkThemePreference::Dracula,
                            ),
                        ],
                    };
                    let providers: Vec<_> = this
                        .profiles
                        .iter()
                        .map(|profile| profile.id.clone())
                        .collect();
                    let choices = options
                        .into_iter()
                        .map(|(label, selected)| Choice {
                            label,
                            selected,
                            ..Default::default()
                        })
                        .collect();
                    let view = cx.new(|cx| ChoiceMenu::new("Choose".into(), choices, cx));
                    let return_focus = window
                        .focused(cx)
                        .unwrap_or_else(|| this.navigation.root_focus.clone());
                    let window_handle = window.window_handle();
                    let subscription = cx.subscribe(&view, move |this, _, event, cx| {
                        if let ChoiceEvent::Selected(index) = event {
                            let index = *index;
                            let provider = providers.get(index).cloned();
                            this.save_setting(
                                |settings| match kind {
                                    ChoiceKind::Provider => {
                                        settings.general.default_provider = provider
                                    }
                                    ChoiceKind::Projects => {
                                        settings.general.alphabetical_projects = index == 1
                                    }
                                    ChoiceKind::Threads => {
                                        settings.general.oldest_threads_first = index == 1
                                    }
                                    ChoiceKind::DarkTheme => {
                                        settings.appearance.dark_theme = if index == 1 {
                                            DarkThemePreference::Dracula
                                        } else {
                                            DarkThemePreference::Synara
                                        }
                                    }
                                },
                                cx,
                            );
                        }
                        if let Some(popup) = this.settings.popup.take() {
                            let focus = popup.return_focus;
                            cx.defer(move |cx| {
                                cx.update_window(window_handle, |_, window, cx| {
                                    window.focus(&focus, cx)
                                })
                                .ok();
                            });
                        }
                        cx.notify();
                    });
                    window.focus(&view.read(cx).focus_handle(cx), cx);
                    this.settings.popup = Some(SettingsPopup {
                        view,
                        position: event.position(),
                        return_focus,
                        _subscription: subscription,
                    });
                    cx.notify();
                }),
            )
            .into_any_element()
    }
    fn toggle(
        &self,
        id: &'static str,
        label: &'static str,
        enabled: bool,
        change: fn(&mut AppSettings),
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        ui::button_shell(id, label, enabled)
            .aria_label(format!("{label}: {}", if enabled { "on" } else { "off" }))
            .w(px(32.))
            .h(px(20.))
            .p(px(2.))
            .rounded_full()
            .border_0()
            .bg(rgb(if enabled {
                palette().focus
            } else {
                palette().border
            }))
            .flex()
            .items_center()
            .when(enabled, |switch| switch.justify_end())
            .child(div().size(px(16.)).rounded_full().bg(rgb(0xf8f8f2)))
            .relative()
            .child(ui::layout_probe(id))
            .when(self.settings.saving, |switch| switch.opacity(0.5))
            .on_click(cx.listener(move |this, _, _, cx| this.save_setting(change, cx)))
            .into_any_element()
    }
    pub(super) fn settings_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(popup) = &self.settings.popup else {
            return div().into_any_element();
        };
        div()
            .id("settings-choice-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if let Some(popup) = this.settings.popup.take() {
                        window.focus(&popup.return_focus, cx);
                    }
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                gpui::anchored()
                    .anchor(gpui::Anchor::TopRight)
                    .position(popup.position)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup.view.clone()),
            )
            .into_any_element()
    }
    pub(super) fn settings_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let info = SECTIONS
            .iter()
            .find(|info| info.section == self.settings.section)
            .unwrap();
        let page = match self.settings.section {
            Section::General => self.general_settings(cx),
            Section::Appearance => self.appearance_settings(cx),
            Section::Profile => self.profile_settings(cx),
            Section::Providers => self.provider_settings(cx),
            Section::Keybindings => self.keybindings_settings(),
            Section::Usage => self.usage_settings(),
            Section::Models => self.model_settings(cx),
            Section::System => self.system_settings(cx),
            Section::Archived => self.archived_settings(cx),
            Section::Behavior => self.chat_settings(cx),
            Section::Notifications => empty("In-app activity", "Running chats show an activity indicator. Permission and input requests appear in the conversation. Desktop notification preferences have not been ported yet.").into_any_element(),
            Section::AppSnap => empty("AppSnap is not available yet", "Capturing another app’s window has not been ported to the native app. You can attach project files from the composer’s Add menu.").into_any_element(),
            Section::Mcp => empty("Managed MCP connections are not available yet", "Your agent can use its own configured tools. Managing shared MCP connections from Synara has not been ported yet.").into_any_element(),
            Section::Skills => empty("Skill management is not available yet", "Your agent’s existing skills remain configured in the agent. A native skill browser has not been ported yet.").into_any_element(),
            Section::Worktrees => empty("Managed worktrees are not available yet", "Open an existing worktree as a project to use it. Creating and cleaning up managed worktrees from this page has not been ported yet.").into_any_element(),
        };
        div()
            .id("settings-view")
            .track_scroll(&self.settings.scroll)
            .mt(px(-30.))
            .text_size(px(15.))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .px_6()
            .pb_10()
            .child(
                div()
                    .w_full()
                    .max_w(px(if self.settings.section == Section::Profile {
                        720.
                    } else {
                        624.
                    }))
                    .mx_auto()
                    .relative()
                    .child(ui::layout_probe("settings-content"))
                    .children((self.settings.section != Section::Profile).then(|| {
                        div()
                            .pt_3()
                            .pb_3()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().text_size(px(20.)).flex_1().child(info.label))
                            .children(
                                matches!(
                                    self.settings.section,
                                    Section::General | Section::Appearance | Section::Behavior
                                )
                                .then(|| {
                                    let section = self.settings.section;
                                    ui::button("settings-restore", "Restore defaults", false)
                                        .border_1()
                                        .border_color(rgb(palette().border))
                                        .bg(gpui::rgba(0))
                                        .text_size(px(13.))
                                        .relative()
                                        .child(ui::layout_probe("settings-restore"))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.settings.saving {
                                                return;
                                            }
                                            if section == Section::Appearance {
                                                this.settings
                                                    .ui_font
                                                    .update(cx, |entry, cx| entry.clear(cx));
                                                this.settings
                                                    .code_font
                                                    .update(cx, |entry, cx| entry.clear(cx));
                                            }
                                            if section == Section::General {
                                                this.reset_environment_default();
                                            }
                                            this.save_setting(
                                                |settings| match section {
                                                    Section::Appearance => {
                                                        settings.appearance =
                                                            AppearanceSettings::default()
                                                    }
                                                    Section::Behavior => {
                                                        settings.chat = ChatSettings::default();
                                                    }
                                                    _ => {
                                                        settings.general =
                                                            GeneralSettings::default()
                                                    }
                                                },
                                                cx,
                                            );
                                        }))
                                }),
                            )
                    }))
                    .children((self.settings.section != Section::Profile).then(|| {
                        div()
                            .text_color(rgb(palette().muted))
                            .line_height(px(22.))
                            .mb_3()
                            .child(info.description)
                    }))
                    .child(page),
            )
            .into_any_element()
    }
    fn general_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let general = &self.settings.value.general;
        let provider = self
            .profiles
            .iter()
            .find(|profile| general.default_provider.as_ref() == Some(&profile.id))
            .or_else(|| self.profiles.first())
            .map_or("Choose agent", |profile| profile.name.as_str());
        div().child(heading("Core defaults"))
            .child(card()
                .child(row("Default provider", "Provider used for your first chat. Following chats reuse your most recent agent.", self.choice_button("default-provider", provider.into(), ChoiceKind::Provider, cx)))
                .child(row("New threads", "Standalone chats get their own local working directory. Use a project’s compose button to work in that project.", "Local"))
                .child(row("Getting started", "Review the available tools and keyboard shortcuts.", ui::button("settings-guide", "Open guide", false).on_click(cx.listener(|this, _, _, cx| this.set_panel(Panel::Help, cx))))))
            .child(heading("Sidebar organization"))
            .child(card().child(row("Project order", "Controls how projects are arranged in the main sidebar.", self.choice_button("project-order", if general.alphabetical_projects { "Alphabetical" } else { "Manual order" }.into(), ChoiceKind::Projects, cx)))
                .child(row("Thread order", "Controls how threads are arranged inside each project and the Chats list.", self.choice_button("thread-order", if general.oldest_threads_first { "Oldest first" } else { "Recently active" }.into(), ChoiceKind::Threads, cx))))
            .child(heading("Sidebar sections"))
            .child(card().child(row("Chats", "Show standalone chats in the sidebar.", self.toggle("show-chats", "Chats", general.show_chats, |s| s.general.show_chats = !s.general.show_chats, cx)))
                .child(row("Studio", "Show Studio in the sidebar switcher.", self.toggle("show-studio", "Studio", general.show_studio, |s| s.general.show_studio = !s.general.show_studio, cx))))
            .child(heading("Environment panel"))
            .child(card()
                .child(row("Open by default", "Open Environment automatically on normal chats. Your last explicit open or hide updates this preference. Restoring the panel never starts a shell or an agent.", self.environment_preference_toggle(cx)))
                .child(row("Workspace layout", "Tabs and split width are remembered. Resetting the layout does not discard editor text or stop running tools.", ui::button("environment-reset", "Reset layout", false)
                    .relative().child(ui::layout_probe("environment-reset"))
                    .on_click(cx.listener(|this, _, _, cx| this.reset_environment_layout(cx))))))
            .children(self.environment.recovery.as_ref().map(|error| div().mt_3().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    fn appearance_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let appearance = &self.settings.value.appearance;
        div()
            .child(heading("Theme"))
            .child(
                div().flex().gap_4().children(
                    [
                        (ThemePreference::System, "System", "theme-system"),
                        (ThemePreference::Light, "Light", "theme-light"),
                        (ThemePreference::Dark, "Dark", "theme-dark"),
                    ]
                    .into_iter()
                    .map(|(theme, label, id)| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .items_center()
                            .child(
                                ui::button_shell(id, label, theme == appearance.theme)
                                    .p(px(3.))
                                    .w_full()
                                    .h(px(140.))
                                    .rounded_xl()
                                    .border_2()
                                    .border_color(rgb(if theme == appearance.theme {
                                        palette().text
                                    } else {
                                        palette().border
                                    }))
                                    .child(theme_preview(theme))
                                    .relative()
                                    .child(ui::layout_probe(id))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.save_setting(|s| s.appearance.theme = theme, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette().muted))
                                    .child(label),
                            )
                    }),
                ),
            )
            .child(
                div().mt_4().child(
                    card()
                        .child(row(
                            "Dark theme",
                            "Used when Dark is selected or your system uses dark mode.",
                            self.choice_button(
                                "dark-theme",
                                if appearance.dark_theme == DarkThemePreference::Dracula {
                                    "Dracula"
                                } else {
                                    "Synara"
                                }
                                .into(),
                                ChoiceKind::DarkTheme,
                                cx,
                            ),
                        ))
                        .child(row("Accent", "", color_swatch(palette().focus)))
                        .child(row("Background", "", color_swatch(palette().canvas)))
                        .child(row("Foreground", "", color_swatch(palette().text)))
                        .child(row(
                            "UI font",
                            "Leave blank to use the system default.",
                            div()
                                .w(px(224.))
                                .relative()
                                .child(ui::layout_probe("ui-font"))
                                .child(self.settings.ui_font.clone()),
                        ))
                        .child(row(
                            "Code font",
                            "Used for code and file editing.",
                            div()
                                .w(px(224.))
                                .relative()
                                .child(ui::layout_probe("code-font"))
                                .child(self.settings.code_font.clone()),
                        ))
                        .child(row(
                            "Typography",
                            "",
                            ui::button("save-fonts", "Apply fonts", false)
                                .relative()
                                .child(ui::layout_probe("save-fonts"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let ui =
                                        this.settings.ui_font.read(cx).text().trim().to_owned();
                                    let code =
                                        this.settings.code_font.read(cx).text().trim().to_owned();
                                    this.save_setting(
                                        |s| {
                                            s.appearance.fonts.ui_family =
                                                (!ui.is_empty()).then_some(ui);
                                            s.appearance.fonts.code_family =
                                                (!code.is_empty()).then_some(code);
                                        },
                                        cx,
                                    );
                                })),
                        ))
                        .child(row(
                            "Reduce motion",
                            "Turn off sliding and fading animations.",
                            self.toggle(
                                "reduce-motion",
                                "Reduce motion",
                                appearance.reduced_motion,
                                |s| s.appearance.reduced_motion = !s.appearance.reduced_motion,
                                cx,
                            ),
                        )),
                ),
            )
            .into_any_element()
    }
    fn profile_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let profile = &self.settings.value.profile;
        let initials: String = profile
            .name
            .split_whitespace()
            .take(2)
            .filter_map(|part| part.chars().next())
            .flat_map(char::to_uppercase)
            .collect();
        let activity = self.settings.activity.as_ref();
        let today = chrono::Utc::now().timestamp_millis().div_euclid(86_400_000);
        div().child(div().pt(px(72.)).pb_5().flex().flex_col().items_center().gap_3()
            .child(div().size(px(64.)).rounded_full().bg(rgb(palette().focus)).text_color(rgb(palette().canvas)).text_size(px(20.)).flex().items_center().justify_center().child(if initials.is_empty() { "SY".into() } else { initials }))
            .child(div().text_size(px(24.)).font_weight(gpui::FontWeight::SEMIBOLD).child(if profile.name.is_empty() { "Your profile".into() } else { profile.name.clone() }))
            .child(div().text_color(rgb(palette().muted)).child(if profile.username.is_empty() { "Synara".into() } else { format!("@{} · Synara", profile.username) })))
            .child(card().flex_row().children([
                ("Reported tokens", activity.and_then(|a| a.tokens).map_or("—".into(), |n| n.to_string())),
                ("Peak day · prompts", activity.map_or("—".into(), |a| a.days.values().max().copied().unwrap_or(0).to_string())),
                ("Total prompts", activity.map_or("—".into(), |a| a.prompts.to_string())),
                ("Active days", activity.map_or("—".into(), |a| a.days.len().to_string())),
                ("Threads", activity.map_or("—".into(), |a| a.threads.to_string())),
            ].into_iter().map(|(label, value)| div().flex_1().p_3().border_r_1().border_color(rgb(palette().border)).flex().flex_col().items_center().gap_2()
                .child(value).child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(label)))))
            .child(heading("Activity"))
            .children(activity.map(|activity| div().w_full().flex().gap(px(3.)).children((0..52).map(|week| {
                div().flex_1().flex().flex_col().gap(px(3.)).children((0..7).map(|day| {
                    let timestamp = today - 363 + week * 7 + day;
                    let count = activity.days.get(&timestamp).copied().unwrap_or(0);
                    div().w_full().h(px(12.)).rounded(px(4.)).bg(rgb(if count > 0 { palette().focus } else { palette().border }))
                }))
            }))))
            .child(div().mt_2().text_size(px(11.)).text_color(rgb(palette().muted)).child("Local activity · last 52 weeks, UTC. Reported tokens sum the latest available totals from each chat."))
            .children(activity.and_then(|activity| activity.error.as_ref()).map(|error| div().mt_2().text_color(rgb(palette().error)).child(format!("Activity could not be fully loaded: {error}"))))
            .child(heading("Edit profile"))
            .child(card().child(row("Display name", "", div().w(px(224.)).relative().child(ui::layout_probe("profile-name")).child(self.settings.name.clone())))
                .child(row("Username", "", div().w(px(224.)).relative().child(ui::layout_probe("profile-username")).child(self.settings.username.clone())))
                .child(row("", "Your profile is saved on this device.", ui::button("save-profile", "Save profile", false).relative().child(ui::layout_probe("save-profile"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let name = this.settings.name.read(cx).text().trim().to_owned();
                        let username = this.settings.username.read(cx).text().trim().trim_start_matches('@').to_owned();
                        this.save_setting(|s| s.profile = ProfileSettings { name, username }, cx);
                    })))))
            .into_any_element()
    }
    fn provider_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .child(heading("Available agents"))
            .child(card().children(self.profiles.iter().map(|profile| {
                row(
                    profile.name.clone(),
                    profile.command.display().to_string(),
                    ui::icon(self.agent_glyph(&profile.id)),
                )
            })))
            .child(
                div().mt_4().child(
                    ui::button("manage-agents", "Manage agents", false)
                        .relative()
                        .child(ui::layout_probe("manage-agents"))
                        .on_click(
                            cx.listener(|this, _, _, cx| this.set_panel(Panel::Registry, cx)),
                        ),
                ),
            )
            .into_any_element()
    }
    fn keybindings_settings(&self) -> gpui::AnyElement {
        card()
            .children(
                [
                    ("Send message", "Enter"),
                    ("New line", "Shift + Enter"),
                    ("Conversation", "Ctrl / ⌘ + 1"),
                    ("Files", "Ctrl / ⌘ + 2"),
                    ("Changes", "Ctrl / ⌘ + 3"),
                    ("Terminal", "Ctrl / ⌘ + 4"),
                    ("Session inspector", "Ctrl / ⌘ + 5"),
                    ("Settings", "Ctrl / ⌘ + 6"),
                    ("Agent providers", "Ctrl / ⌘ + 7"),
                    ("Remote workspaces", "Ctrl / ⌘ + 8"),
                    ("Kanban", "Ctrl / ⌘ + 9"),
                    ("Close menu", "Escape"),
                ]
                .into_iter()
                .map(|(label, key)| {
                    row(
                        label,
                        "",
                        div()
                            .text_size(px(13.))
                            .text_color(rgb(palette().muted))
                            .child(key),
                    )
                }),
            )
            .into_any_element()
    }
    fn usage_settings(&self) -> gpui::AnyElement {
        let usage = self.thread.as_ref().map(|thread| &thread.usage);
        card()
            .child(row(
                "Input tokens",
                "Reported for the selected chat.",
                usage
                    .and_then(|u| u.input_tokens)
                    .map_or("Not reported".into(), |n| n.to_string()),
            ))
            .child(row(
                "Output tokens",
                "",
                usage
                    .and_then(|u| u.output_tokens)
                    .map_or("Not reported".into(), |n| n.to_string()),
            ))
            .child(row(
                "Context",
                "",
                usage
                    .and_then(|u| u.context_used.zip(u.context_limit))
                    .map_or("Not reported".into(), |(used, limit)| {
                        format!("{used} / {limit}")
                    }),
            ))
            .child(row(
                "Account limits",
                "Account quotas are not provided by the connected agent.",
                "Not available",
            ))
            .into_any_element()
    }
    fn model_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let models = self
            .thread
            .as_ref()
            .map(|thread| &thread.configuration.models);
        div()
            .children(models.filter(|models| !models.is_empty()).map(|models| {
                card().children(models.iter().map(|model| row(model.label.clone(), "", "")))
            }))
            .children(models.is_none_or(|models| models.is_empty()).then(|| {
                empty(
                    "No models loaded",
                    "Connect to an agent from the composer to load its available models.",
                )
            }))
            .child(div().mt_4().child(
                ui::button("models-back", "Open chat", false).on_click(
                    cx.listener(|this, _, _, cx| this.set_panel(Panel::Conversation, cx)),
                ),
            ))
            .into_any_element()
    }
    fn system_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        card()
            .children(
                [
                    (
                        Panel::Inspector,
                        "Session inspector",
                        "Review the current agent session and reconnect if needed.",
                    ),
                    (
                        Panel::Remote,
                        "Remote workspaces",
                        "Open a workspace using a configured SSH connection.",
                    ),
                    (
                        Panel::Registry,
                        "Agent tools",
                        "Install or configure an agent.",
                    ),
                    (
                        Panel::Help,
                        "About Synara",
                        "Application help, licenses, and keyboard shortcuts.",
                    ),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (panel, title, detail))| {
                    row(
                        title,
                        detail,
                        ui::button(("system-tool", index), "Open", false)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_panel(panel, cx))),
                    )
                }),
            )
            .into_any_element()
    }
    fn archived_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let tasks: Vec<_> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| task.state == TaskState::Archived)
            .collect();
        if tasks.is_empty() {
            return empty(
                "No archived threads",
                "Threads you archive will appear here.",
            )
            .into_any_element();
        }
        card()
            .children(tasks.into_iter().map(|task| {
                let id = task.id;
                row(
                    task.title.clone(),
                    "",
                    ui::button(
                        SharedString::from(format!("restore-{id}")),
                        "Restore",
                        false,
                    )
                    .on_click(cx.listener(move |this, _, _, _| {
                        let workspace = this.controller.workspace.clone();
                        this.job(async move {
                            workspace.unarchive_task(id).await?;
                            Ok(Update::Catalog(workspace.catalog().await?))
                        });
                    })),
                )
            }))
            .into_any_element()
    }
}
fn color_swatch(color: u32) -> gpui::Div {
    div()
        .w(px(176.))
        .h(px(32.))
        .rounded_lg()
        .border_1()
        .border_color(rgb(palette().border))
        .px_2()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(20.))
                .rounded_full()
                .bg(rgb(color))
                .border_1()
                .border_color(rgb(palette().muted)),
        )
        .child(format!("#{color:06X}"))
}
fn theme_preview(theme: ThemePreference) -> gpui::Div {
    div()
        .size_full()
        .rounded_lg()
        .overflow_hidden()
        .relative()
        .child(theme_preview_scene(theme == ThemePreference::Dark))
        .when(theme == ThemePreference::System, |preview| {
            preview.child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .w(gpui::relative(0.5))
                    .h_full()
                    .overflow_hidden()
                    .child(
                        theme_preview_scene(true)
                            .absolute()
                            .right_0()
                            .w(gpui::relative(2.)),
                    ),
            )
        })
}

fn theme_preview_scene(dark: bool) -> gpui::Div {
    let background = if dark { 0x62615f } else { 0xe5e4e4 };
    let window = if dark { 0x282828 } else { 0xf7f7f7 };
    let panel = if dark { 0x383838 } else { 0xffffff };
    let line = if dark { 0x737373 } else { 0xd7d7d7 };
    div().size_full().bg(rgb(background)).px_4().pt_5().child(
        div()
            .h_full()
            .rounded_t_lg()
            .bg(rgb(window))
            .pt_4()
            .px_4()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(4.))
                    .w_16()
                    .rounded_full()
                    .bg(rgb(line)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(2.))
                    .w_20()
                    .rounded_full()
                    .bg(rgb(line)),
            )
            .child(
                div()
                    .mt_2()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .rounded_t_lg()
                    .bg(rgb(panel))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children((0..4).map(|index| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .h(px(4.))
                                    .w(px(if index == 0 { 38. } else { 30. }))
                                    .rounded_full()
                                    .bg(rgb(line)),
                            )
                            .child(div().h(px(1.)).w_full().bg(rgb(line)).opacity(0.25))
                    })),
            ),
    )
}
