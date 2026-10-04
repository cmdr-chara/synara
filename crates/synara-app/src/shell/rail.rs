//! Upstream rail layout (`railShellStore.ts` + `AppRail.tsx` + the rail half
//! of `Sidebar.tsx`): a fixed icon strip owns section switching while the
//! panel column keeps the project/thread lists, the Automations list, and the
//! hub index. Item order, hidden items and pinned Space/project shortcuts are
//! app settings; the active item reconciles with the surface after each
//! navigation.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::{Div, Stateful, rgba};

/// Upstream `--app-rail-width: 3.25rem` at the 16px root.
const RAIL_WIDTH: f32 = 52.;
const RAIL_BUTTON: f32 = 36.;
/// Page size shared with the classic sidebar's project/thread paging.
const RAIL_PAGE_SIZE: usize = 5;

/// `railShellStore` — which item is active, which panel item owns the panel
/// column, which project the Spaces panel is drilled into, and the rail's own
/// "…"/Customize popover state. Upstream persists the first three in
/// sessionStorage; a native window's lifetime is the same scope.
pub(super) struct RailState {
    pub(super) active_item: RailItemId,
    pub panel_view: RailPanelItemId,
    pub spaces_project_id: Option<ProjectId>,
    pub spaces_page: usize,
    pub more_open: bool,
    pub customize_open: bool,
}
impl RailState {
    pub fn new() -> Self {
        Self {
            active_item: RailItemId::Panel(RailPanelItemId::Home),
            panel_view: RailPanelItemId::Home,
            spaces_project_id: None,
            spaces_page: 0,
            more_open: false,
            customize_open: false,
        }
    }
}

/// `RAIL_ITEM_GLYPH_NAMES` — Central icon basenames (idle outline / active fill).
fn rail_item_glyph(id: RailItemId) -> &'static str {
    match id {
        RailItemId::Panel(RailPanelItemId::Home) => "home-roof-door",
        RailItemId::Panel(RailPanelItemId::Spaces) => "folders",
        RailItemId::Route(RailRouteItemId::Kanban) => "columns-3-wide",
        RailItemId::Route(RailRouteItemId::PullRequests) => "pull-request",
        RailItemId::Route(RailRouteItemId::Automations) => "clock",
        RailItemId::Route(RailRouteItemId::Studio) => "images-1",
        RailItemId::Route(RailRouteItemId::Settings) => "settings-gear-4",
    }
}
fn rail_item_label(id: RailItemId) -> &'static str {
    match id {
        RailItemId::Panel(RailPanelItemId::Home) => "Home",
        RailItemId::Panel(RailPanelItemId::Spaces) => "Spaces",
        RailItemId::Route(RailRouteItemId::Kanban) => "Tasks",
        RailItemId::Route(RailRouteItemId::PullRequests) => "Pull requests",
        RailItemId::Route(RailRouteItemId::Automations) => "Automations",
        RailItemId::Route(RailRouteItemId::Studio) => "Studio",
        RailItemId::Route(RailRouteItemId::Settings) => "Settings",
    }
}
const RAIL_MORE_GLYPH: &str = "dot-grid-1x3-horizontal";

impl Shell {
    /// Upstream `useSidebarLayout` resolves rail for the setting on a desktop
    /// form factor; the native shell has no mobile branch.
    pub(super) fn rail_enabled(&self) -> bool {
        self.settings.value.general.sidebar_layout == SidebarLayout::Rail
    }

    /// Upstream `railHidesPanel`: full-width items (Kanban, Pull requests)
    /// collapse the panel column but never the rail itself.
    fn rail_panel_allowed(&self) -> bool {
        !self.rail_enabled() || rail_item_shows_panel(self.rail.active_item)
    }

    /// The destination-owning panel as a route rail item (upstream
    /// `railItemForPathname` over the surface's route).
    fn rail_destination(&self) -> Option<RailRouteItemId> {
        match self.panel {
            Panel::Kanban => Some(RailRouteItemId::Kanban),
            Panel::PullRequests => Some(RailRouteItemId::PullRequests),
            Panel::Automations => Some(RailRouteItemId::Automations),
            Panel::Settings => Some(RailRouteItemId::Settings),
            _ => None,
        }
    }

    /// Upstream `reconcile` + `reconcileActiveRailItem`: re-sync the active
    /// item after navigation and drop the drilled Spaces project once the
    /// catalog no longer has it.
    pub(super) fn reconcile_rail(&mut self, cx: &mut Context<Self>) {
        if !self.rail_enabled() {
            return;
        }
        let destination = self.rail_destination();
        let next = reconcile_active_rail_item(
            self.rail.active_item,
            destination,
            destination.is_none() && self.navigation.studio,
            self.rail.panel_view,
        );
        let mut changed = next != self.rail.active_item;
        self.rail.active_item = next;
        if self
            .rail
            .spaces_project_id
            .is_some_and(|id| !self.catalog.projects.iter().any(|project| project.id == id))
        {
            self.rail.spaces_project_id = None;
            changed = true;
        }
        self.sync_rail_drawer(cx);
        if changed {
            cx.notify();
        }
    }

    /// The panel column's effective open state (upstream `resolvedSidebarOpen`):
    /// the user's toggle AND the active rail item owning a panel.
    pub(super) fn rail_drawer_open(&self) -> bool {
        self.navigation.visible && self.rail_panel_allowed()
    }

    fn sync_rail_drawer(&mut self, cx: &mut Context<Self>) {
        let open = self.rail_drawer_open();
        self.navigation
            .drawer
            .set_open(open, std::time::Instant::now(), cx.reduce_motion());
    }

    /// Upstream `handleSidebarOpenChange`: reopening over a full-width item
    /// selects the current panel item first, then opens.
    pub(super) fn set_sidebar_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if open && self.rail_enabled() && !self.rail_panel_allowed() {
            let view = self.rail.panel_view;
            self.select_rail_panel_item(view, cx);
        }
        self.navigation.visible = open;
        self.sync_rail_drawer(cx);
        cx.notify();
    }

    /// Upstream `selectPanelItem`/`selectRouteItem`: store-level select, no
    /// navigation.
    fn select_rail_item(&mut self, item: RailItemId, cx: &mut Context<Self>) {
        self.rail.active_item = item;
        if let RailItemId::Panel(view) = item {
            self.rail.panel_view = view;
        }
        self.sync_rail_drawer(cx);
        cx.notify();
    }
    pub(super) fn select_rail_panel_item(&mut self, view: RailPanelItemId, cx: &mut Context<Self>) {
        self.select_rail_item(RailItemId::Panel(view), cx);
    }

    /// Upstream `isOnThreadsSection`: Home/Spaces content only makes sense
    /// over the thread surface, so full-width sections bounce back to it.
    fn rail_on_threads_surface(&self) -> bool {
        !self.navigation.studio
            && !matches!(
                self.panel,
                Panel::Kanban
                    | Panel::PullRequests
                    | Panel::Automations
                    | Panel::Settings
                    | Panel::Hubs
            )
    }

    /// Upstream `handleSidebarViewChange("threads")`.
    fn rail_threads_view(&mut self, cx: &mut Context<Self>) {
        if self.rail_on_threads_surface() {
            return;
        }
        if self.navigation.studio {
            self.switch_mode(false, cx);
        } else {
            self.set_panel(Panel::Conversation, cx);
        }
    }

    /// Upstream `handleRailItemSelect` — one rail button's whole behavior.
    pub(super) fn select_rail_orderable(
        &mut self,
        id: RailOrderableItemId,
        cx: &mut Context<Self>,
    ) {
        match id.item() {
            RailItemId::Panel(view) => {
                self.select_rail_panel_item(view, cx);
                self.rail_threads_view(cx);
            }
            RailItemId::Route(RailRouteItemId::Studio) => {
                self.open_rail_studio(cx);
            }
            RailItemId::Route(route) => {
                self.rail.active_item = RailItemId::Route(route);
                let panel = match route {
                    RailRouteItemId::Kanban => Panel::Kanban,
                    RailRouteItemId::PullRequests => Panel::PullRequests,
                    RailRouteItemId::Automations => Panel::Automations,
                    RailRouteItemId::Settings => Panel::Settings,
                    RailRouteItemId::Studio => unreachable!(),
                };
                self.set_panel(panel, cx);
            }
        }
        self.sync_rail_drawer(cx);
    }

    /// Upstream `openRailStudio` — absent when the Studio section is disabled.
    fn open_rail_studio(&mut self, cx: &mut Context<Self>) {
        if !self.settings.value.general.show_studio {
            return;
        }
        self.rail.active_item = RailItemId::STUDIO;
        self.switch_mode(true, cx);
        self.sync_rail_drawer(cx);
    }

    /// Space shortcut: Home panel over the tapped Space. Switching Space lands
    /// on the thread view, like the strip tabs already do.
    fn select_rail_space(&mut self, space_id: Option<String>, cx: &mut Context<Self>) {
        self.select_rail_panel_item(RailPanelItemId::Home, cx);
        if self.organization.value.active != space_id {
            self.select_space(space_id, cx);
        }
        self.rail_threads_view(cx);
    }

    /// Upstream `openSpacesProject`: Spaces panel drilled into one project.
    fn select_rail_project(&mut self, project_id: ProjectId, cx: &mut Context<Self>) {
        self.select_rail_panel_item(RailPanelItemId::Spaces, cx);
        self.rail.spaces_project_id = Some(project_id);
        self.rail.spaces_page = 0;
        self.rail_threads_view(cx);
    }

    /// Upstream `closeSpacesProject`.
    fn close_rail_spaces_project(&mut self, cx: &mut Context<Self>) {
        self.rail.spaces_project_id = None;
        cx.notify();
    }

    /// Persisted shortcut keys toggle through app settings (upstream
    /// `toggleRailShortcutKey` via `updateSettings`).
    fn toggle_rail_shortcut(&mut self, key: String, cx: &mut Context<Self>) {
        self.save_setting(
            move |settings| {
                settings.general.rail_shortcuts =
                    toggle_rail_shortcut_key(&settings.general.rail_shortcuts, &key);
            },
            cx,
        );
    }

    /// `railShortcuts`/`railItemOrder`/`hiddenRailItems` normalized for the
    /// current catalog — the same shapes the Customize rows consume.
    fn rail_config(
        &self,
    ) -> (
        Vec<RailOrderableItemId>,
        HashSet<RailOrderableItemId>,
        Vec<RailShortcut>,
    ) {
        let order = normalize_rail_item_order(&self.settings.value.general.rail_item_order);
        let hidden: HashSet<_> =
            normalize_hidden_rail_items(&self.settings.value.general.hidden_rail_items)
                .into_iter()
                .collect();
        let space_ids: HashSet<String> = self
            .organization
            .value
            .spaces
            .iter()
            .map(|space| space.id.clone())
            .collect();
        let project_ids: HashSet<ProjectId> = self
            .catalog
            .projects
            .iter()
            .map(|project| project.id)
            .collect();
        let shortcuts = resolve_rail_shortcuts(
            &self.settings.value.general.rail_shortcuts,
            &space_ids,
            &project_ids,
        );
        (order, hidden, shortcuts)
    }

    /// Upstream `automationAttentionCount`: runs whose status needs attention.
    /// (Upstream also counts unread triage results — the native ledger has no
    /// result payload yet.)
    fn rail_automation_badge(&self) -> Option<u32> {
        let count = self
            .automations
            .ledger()
            .runs
            .iter()
            .filter(|run| {
                matches!(
                    run.status,
                    AutomationRunStatus::Failed
                        | AutomationRunStatus::Cancelled
                        | AutomationRunStatus::Interrupted
                )
            })
            .count();
        (count > 0).then_some(u32::try_from(count).unwrap_or(u32::MAX))
    }

    /// One rail strip button (upstream `AppRailButton`): 36px, rounded, muted
    /// outline glyph idle, filled accent glyph + panel tint active, a small
    /// accent dot top-right when a badge count is supplied.
    fn rail_button(
        &self,
        id: impl Into<gpui::ElementId>,
        label: SharedString,
        icon: gpui::AnyElement,
        selected: bool,
        badge: Option<u32>,
        activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let aria = match badge {
            Some(count) => format!(
                "{label}: {count} {} attention",
                if count == 1 {
                    "item needs"
                } else {
                    "items need"
                }
            )
            .into(),
            None => label.clone(),
        };
        let tooltip = label.clone();
        div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(aria)
            .aria_selected(selected)
            .tab_index(0)
            .size(px(RAIL_BUTTON))
            .rounded(px(8.))
            .border_1()
            .border_color(rgba(0))
            .bg(if selected {
                rgb(palette().selected)
            } else {
                rgba(0)
            })
            .text_color(rgb(if selected {
                palette().text
            } else {
                palette().muted
            }))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(palette().hover)))
            .active(|style| style.bg(rgb(palette().selected)))
            .focus_visible(|style| style.border_color(rgb(palette().focus)))
            .flex()
            .items_center()
            .justify_center()
            .relative()
            .child(icon)
            .children(badge.is_some().then(|| {
                div()
                    .absolute()
                    .top(px(4.))
                    .right(px(4.))
                    .size(px(6.))
                    .rounded_full()
                    .bg(rgb(palette().focus))
            }))
            .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(tooltip.clone())).into())
            .on_click(move |_, window, cx| {
                activate(&(), window, cx);
                cx.stop_propagation();
            })
    }

    /// Small icon control for dynamically-named rows where `ui::chrome_button`'s
    /// static id cannot be formed.
    fn rail_glyph_button(
        &self,
        id: SharedString,
        label: &'static str,
        icon: gpui::AnyElement,
        disabled: bool,
        activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(label)
            .tab_index(0)
            .size(px(24.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_1()
            .border_color(rgba(0))
            .text_color(rgb(palette().muted))
            .cursor_pointer()
            .hover(|style| {
                style
                    .bg(rgb(palette().hover))
                    .text_color(rgb(palette().text))
            })
            .focus_visible(|style| style.border_color(rgb(palette().focus)))
            .when(disabled, |el| el.opacity(0.4).cursor_default())
            .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(label.into())).into())
            .on_click(move |_, window, cx| {
                if !disabled {
                    activate(&(), window, cx);
                }
            })
            .child(icon)
    }

    fn rail_central_button(
        &self,
        id: impl Into<gpui::ElementId>,
        label: &'static str,
        name: &'static str,
        selected: bool,
        badge: Option<u32>,
        activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let icon = if selected {
            ui::central_fill_icon(name)
        } else {
            ui::central_icon(name)
        }
        .size(px(18.))
        .text_color(rgb(if selected {
            palette().text
        } else {
            palette().muted
        }))
        .into_any_element();
        self.rail_button(id, label.into(), icon, selected, badge, activate)
    }

    /// The rail strip itself (upstream `AppRail`): ordered items, shortcut
    /// divider + shortcuts, then the bottom cluster (Help, Settings) and the
    /// "…" menu button.
    pub(super) fn rail_strip(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (order, hidden, shortcuts) = self.rail_config();
        let studio_available = self.settings.value.general.show_studio;
        let visible =
            build_rail_item_order(&order, &hidden, self.rail.active_item, studio_available);
        let active_shortcut = resolve_active_rail_shortcut_key(
            self.rail.active_item,
            self.organization.value.active.as_deref(),
            self.rail.spaces_project_id,
            &shortcuts,
        );
        let automation_badge = self.rail_automation_badge();
        let items: Vec<gpui::AnyElement> = visible
            .iter()
            .map(|id| {
                let id = *id;
                let item = id.item();
                let selected = self.rail.active_item == item;
                let badge = (item == RailItemId::AUTOMATIONS)
                    .then_some(automation_badge)
                    .flatten();
                self.rail_central_button(
                    SharedString::from(format!("rail-item-{}", id.name())),
                    rail_item_label(item),
                    rail_item_glyph(item),
                    selected,
                    badge,
                    cx.listener(move |this, _: &(), _, cx| {
                        this.select_rail_orderable(id, cx);
                        this.rail.more_open = false;
                    }),
                )
                .into_any_element()
            })
            .collect();
        let shortcut_buttons: Vec<gpui::AnyElement> = shortcuts
            .iter()
            .map(|shortcut| {
                let key = shortcut.key().to_owned();
                let selected = active_shortcut.as_deref() == Some(shortcut.key());
                match shortcut {
                    RailShortcut::Space { space_id, .. } => {
                        let space_id = space_id.clone();
                        let (name, icon) = match space_id.as_deref() {
                            None => (
                                "Void".to_owned(),
                                if selected {
                                    ui::central_fill_icon("black-hole")
                                } else {
                                    ui::central_icon("black-hole")
                                }
                                .into_any_element(),
                            ),
                            Some(id) => {
                                let space = self
                                    .organization
                                    .value
                                    .spaces
                                    .iter()
                                    .find(|space| space.id == id);
                                (
                                    space
                                        .map(|space| space.name.clone())
                                        .unwrap_or_else(|| "Unknown space".to_owned()),
                                    ui::icon(
                                        space
                                            .map(|space| space.symbol)
                                            .map(super::organization::symbol_glyph)
                                            .unwrap_or(Glyph::Folder),
                                    )
                                    .into_any_element(),
                                )
                            }
                        };
                        self.rail_button(
                            SharedString::from(format!("rail-shortcut-{key}")),
                            name.into(),
                            icon,
                            selected,
                            None,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.select_rail_space(space_id.clone(), cx)
                            }),
                        )
                        .into_any_element()
                    }
                    RailShortcut::Project { project_id, .. } => {
                        let project_id = *project_id;
                        let project = self
                            .catalog
                            .projects
                            .iter()
                            .find(|project| project.id == project_id);
                        let Some(project) = project else {
                            return div().into_any_element();
                        };
                        let name = self.project_name(project);
                        let icon = self.project_icon(
                            project,
                            false,
                            project_ui::ProjectGlyphPresentation::Favicon,
                        );
                        self.rail_button(
                            SharedString::from(format!("rail-shortcut-{key}")),
                            name,
                            icon,
                            selected,
                            None,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.select_rail_project(project_id, cx)
                            }),
                        )
                        .into_any_element()
                    }
                }
            })
            .collect();
        // "…" stands for Studio while it is open but has no rail button.
        let more_active = self.rail.active_item == RailItemId::STUDIO
            && !visible.contains(&RailOrderableItemId::Studio);
        div()
            .id("app-rail")
            .role(gpui::Role::Navigation)
            .aria_label("Primary")
            .w(px(RAIL_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .pt(px(10.))
            .pb(px(10.))
            .bg(ui::surface(rail_shell_tone()))
            .child(ui::layout_probe("app-rail"))
            .children(items)
            .children((!shortcut_buttons.is_empty()).then(|| {
                div()
                    .my(px(2.))
                    .h(px(1.))
                    .w(px(20.))
                    .bg(rail_inset_border())
            }))
            .children(shortcut_buttons)
            .child(self.rail_central_button(
                "rail-more",
                "More",
                RAIL_MORE_GLYPH,
                more_active,
                None,
                cx.listener(|this, _: &(), _, cx| {
                    this.rail.customize_open = false;
                    this.rail.more_open = !this.rail.more_open;
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .mt_auto()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.))
                    .child(self.rail_button(
                        "rail-help",
                        "Help".into(),
                        ui::icon(Glyph::Help).size(px(18.)).into_any_element(),
                        self.panel == Panel::Help,
                        None,
                        cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Help, cx)),
                    ))
                    .child(self.rail_central_button(
                        "rail-settings",
                        "Settings",
                        rail_item_glyph(RailItemId::SETTINGS),
                        self.rail.active_item == RailItemId::SETTINGS,
                        None,
                        cx.listener(|this, _: &(), _, cx| {
                            this.rail.active_item = RailItemId::SETTINGS;
                            this.set_panel(Panel::Settings, cx);
                        }),
                    )),
            )
            .into_any_element()
    }

    /// The panel column's content under the rail layout (upstream's rail
    /// branches in `Sidebar`): Settings and Automations own their panels, a
    /// Studio surface shows the hub index, the Spaces view shows the grouped
    /// project list or one project's threads, and Home shows the threads
    /// surface for the active Space.
    pub(super) fn rail_panel_content(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.panel == Panel::Settings {
            return self.settings_sidebar(cx);
        }
        if self.panel == Panel::Automations {
            return self.rail_automations_panel(cx);
        }
        if self.navigation.studio {
            return self.hub_sidebar(cx);
        }
        if self.rail.panel_view == RailPanelItemId::Spaces {
            return self.rail_spaces_panel(cx);
        }
        self.sidebar(cx)
    }

    /// Upstream `RailAutomationsPanel`: title, primary "New automation" action,
    /// then active definitions over paused ones; a row opens the editor.
    fn rail_automations_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut active: Vec<&AutomationDefinition> = Vec::new();
        let mut paused: Vec<&AutomationDefinition> = Vec::new();
        for definition in &self.automations.ledger().definitions {
            if definition.enabled {
                active.push(definition);
            } else {
                paused.push(definition);
            }
        }
        let attention: HashSet<AutomationId> = self
            .automations
            .ledger()
            .runs
            .iter()
            .filter(|run| {
                matches!(
                    run.status,
                    AutomationRunStatus::Failed
                        | AutomationRunStatus::Cancelled
                        | AutomationRunStatus::Interrupted
                )
            })
            .map(|run| run.definition.id)
            .collect();
        let row = |definition: &AutomationDefinition| {
            let id = definition.id;
            let project = self
                .catalog
                .projects
                .iter()
                .find(|project| project.id == definition.project_id)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "Unavailable project".to_owned());
            let flagged = attention.contains(&id);
            div()
                .id(SharedString::from(format!("rail-automation-{id}")))
                .role(gpui::Role::Button)
                .aria_label(definition.title.clone())
                .tab_index(0)
                .w_full()
                .px_2()
                .py(px(6.))
                .rounded_md()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(palette().hover)))
                .focus_visible(|style| style.bg(rgb(palette().hover)))
                .flex()
                .items_center()
                .gap(px(8.))
                .relative()
                .child(ui::icon(Glyph::Clock).size(px(14.)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().text_ellipsis().child(definition.title.clone()))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette().muted))
                                .text_ellipsis()
                                .child(format!(
                                    "{project} · {}{}",
                                    definition.schedule.label(),
                                    if definition.enabled { "" } else { " · Paused" }
                                )),
                        ),
                )
                .children(flagged.then(|| {
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(rgb(palette().focus))
                        .flex_shrink_0()
                }))
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.open_automation_for_review(id, cx);
                }))
                .into_any_element()
        };
        div()
            .id("rail-automations-panel")
            .w(px(ui::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(ui::surface(palette().sidebar))
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .pl_4()
                    .pr_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Automations"),
                    )
                    .child(
                        ui::button("rail-automations-new", "New automation", false)
                            .relative()
                            .child(ui::layout_probe("rail-automations-new"))
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.open_new_automation(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .id("rail-automation-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_4()
                    .children(
                        (self.automations.loaded() && active.is_empty() && paused.is_empty()).then(
                            || {
                                div()
                                    .px_2()
                                    .py_2()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette().muted))
                                    .child("No automations yet")
                            },
                        ),
                    )
                    .children(active.iter().map(|definition| row(definition)))
                    .children(
                        (!active.is_empty() && !paused.is_empty())
                            .then(|| div().my(px(4.)).h(px(1.)).mx_2().bg(rgb(palette().border))),
                    )
                    .children(paused.iter().map(|definition| row(definition))),
            )
            .into_any_element()
    }

    /// Upstream `RailSpacesPanel` level two: Back + the drilled project's name,
    /// its row toolbar, and its expanded thread list with own paging.
    fn rail_spaces_project(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(project_id) = self.rail.spaces_project_id else {
            return div().into_any_element();
        };
        let Some(project) = self
            .catalog
            .projects
            .iter()
            .find(|project| project.id == project_id)
        else {
            return div().into_any_element();
        };
        let name = self.project_name(project);
        let mut threads: Vec<Task> = self
            .catalog
            .tasks
            .iter()
            .filter(|task| {
                task.scope == TaskScope::Project
                    && task.project_id == project_id
                    && task.state != TaskState::Archived
            })
            .cloned()
            .collect();
        threads.sort_by_key(|task| std::cmp::Reverse(task.updated_at_ms));
        if self.settings.value.general.oldest_threads_first {
            threads.reverse();
        }
        let shown = (self.rail.spaces_page + 1) * RAIL_PAGE_SIZE;
        let count = threads.len();
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .flex_1()
            .child(
                div()
                    .px_2()
                    .pt(px(3.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        ui::chrome_button(
                            "rail-spaces-back",
                            "Back to Spaces",
                            Glyph::Back,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.close_rail_spaces_project(cx)),
                        )
                        .size(px(24.)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(name.clone()),
                    )
                    .child(
                        ui::chrome_button(
                            "rail-project-edit",
                            "Edit project",
                            Glyph::Pencil,
                            false,
                            cx.listener(move |this, _: &(), window, cx| {
                                this.open_project_edit(project_id, window, cx);
                            }),
                        )
                        .size(px(24.)),
                    )
                    .child(
                        ui::chrome_button(
                            "rail-project-new-chat",
                            "New project thread",
                            Glyph::Compose,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.new_project_chat(project_id, cx);
                            }),
                        )
                        .size(px(24.)),
                    ),
            )
            .child(
                div()
                    .id("rail-spaces-threads")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_4()
                    .children((count == 0).then(|| {
                        div()
                            .px_2()
                            .py_2()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("No threads yet")
                    }))
                    .children(
                        threads
                            .iter()
                            .take(shown)
                            .map(|task| self.thread_row(task, false, cx)),
                    )
                    .children((count > shown).then(|| {
                        ui::action(
                            "rail-spaces-more",
                            "Show more",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.rail.spaces_page += 1;
                                cx.notify();
                            }),
                        )
                        .pl(px(30.))
                        .h(px(32.))
                        .text_color(rgb(palette().muted))
                        .into_any_element()
                    })),
            )
            .into_any_element()
    }

    /// Upstream `RailSpacesPanel` level one: Space sections in picker order,
    /// each with its add-project affordance and rows that drill in.
    fn rail_spaces_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.rail.spaces_project_id.is_some() {
            return div()
                .id("rail-spaces-panel")
                .w(px(ui::SIDEBAR_WIDTH))
                .h_full()
                .flex_shrink_0()
                .min_h_0()
                .flex()
                .flex_col()
                .bg(ui::surface(palette().sidebar))
                .child(self.rail_spaces_project(cx))
                .into_any_element();
        }
        let space_ids: Vec<String> = self
            .organization
            .value
            .spaces
            .iter()
            .map(|space| space.id.clone())
            .collect();
        let projects: Vec<&Project> = self
            .catalog
            .projects
            .iter()
            .filter(|project| !self.is_chat_workspace(project))
            .collect();
        let organization = &self.organization.value;
        let sections = build_rail_spaces_sections(
            projects,
            &space_ids,
            |id| match id {
                None => "Void".to_owned(),
                Some(id) => organization
                    .spaces
                    .iter()
                    .find(|space| space.id == id)
                    .map(|space| space.name.clone())
                    .unwrap_or_else(|| "Unknown space".to_owned()),
            },
            organization.active.as_deref(),
            |project: &&Project| organization.space_for(project.id).map(str::to_owned),
            "Void",
        );
        let disabled = !self.organization.loaded || self.organization.saving;
        div()
            .id("rail-spaces-panel")
            .w(px(ui::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(ui::surface(palette().sidebar))
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .pl_4()
                    .pr_2()
                    .flex()
                    .items_center()
                    .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Spaces")),
            )
            .child(
                div()
                    .id("rail-spaces-sections")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_4()
                    .children(sections.into_iter().map(|section| {
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px_2()
                                    .pt_2()
                                    .pb_1()
                                    .flex()
                                    .items_center()
                                    .group("rail-space-section")
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_ellipsis()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette().muted))
                                            .child(section.name.clone()),
                                    )
                                    .child(
                                        self.rail_glyph_button(
                                            SharedString::from(format!(
                                                "rail-space-add-{}",
                                                section.key
                                            )),
                                            "Add project",
                                            ui::icon(Glyph::Plus).into_any_element(),
                                            disabled,
                                            cx.listener(|this, _: &(), _, cx| {
                                                this.browse_workspace(cx)
                                            }),
                                        )
                                        .opacity(0.)
                                        .group_hover("rail-space-section", |style| {
                                            style.opacity(1.)
                                        })
                                        .focus_visible(|style| style.opacity(1.)),
                                    ),
                            )
                            .children(section.items.is_empty().then(|| {
                                div()
                                    .px_2()
                                    .py_2()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette().muted))
                                    .child(if section.space_id.is_none() {
                                        "Projects you haven't filed into a Space appear here."
                                    } else {
                                        "No projects yet"
                                    })
                            }))
                            .children(section.items.into_iter().map(|project| {
                                let id = project.id;
                                let running = self.catalog.tasks.iter().any(|task| {
                                    task.project_id == id
                                        && (task.state == TaskState::Running
                                            || self.busy.contains(&task.id))
                                });
                                let waiting = self.catalog.tasks.iter().any(|task| {
                                    task.project_id == id && task.state == TaskState::Waiting
                                });
                                let icon = self.project_icon(
                                    project,
                                    false,
                                    project_ui::ProjectGlyphPresentation::Badge,
                                );
                                ui::action_icon(
                                    SharedString::from(format!("rail-project-{id}")),
                                    self.project_name(project),
                                    Some(icon),
                                    false,
                                    cx.listener(move |this, _: &(), _, cx| {
                                        this.select_rail_project(id, cx)
                                    }),
                                )
                                .h(px(32.))
                                .children((running || waiting).then(|| {
                                    div()
                                        .size(px(5.))
                                        .rounded_full()
                                        .bg(rgb(if waiting {
                                            palette().error
                                        } else {
                                            palette().focus
                                        }))
                                        .flex_shrink_0()
                                }))
                                .into_any_element()
                            }))
                    })),
            )
            .into_any_element()
    }

    /// Upstream `AppRailMoreMenu`: Studio (while its section is enabled), the
    /// Spaces and projects shortcut checkboxes, and the Customize entry.
    pub(super) fn rail_more_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.rail.more_open {
            return div().into_any_element();
        }
        let (_, _, shortcuts) = self.rail_config();
        let pinned: HashSet<&str> = shortcuts.iter().map(|s| s.key()).collect();
        let section_label = |label: &str| {
            div()
                .px_2()
                .pt_2()
                .pb_1()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(label.to_owned())
        };
        let check_row = |id: SharedString,
                         label: String,
                         checked: bool,
                         key: String,
                         cx: &mut Context<Self>|
         -> gpui::AnyElement {
            let toggle_key = key.clone();
            ui::action(
                id,
                label,
                if checked { Some(Glyph::Check) } else { None },
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    this.toggle_rail_shortcut(toggle_key.clone(), cx);
                }),
            )
            .h(px(28.))
            .into_any_element()
        };
        let spaces: Vec<gpui::AnyElement> = std::iter::once((None, "Void".to_owned()))
            .chain(
                self.organization
                    .value
                    .spaces
                    .iter()
                    .map(|space| (Some(space.id.clone()), space.name.clone())),
            )
            .map(|(id, name)| {
                let key = rail_space_shortcut_key(id.as_deref());
                check_row(
                    SharedString::from(format!("rail-more-{key}")),
                    name,
                    pinned.contains(key.as_str()),
                    key,
                    cx,
                )
            })
            .collect();
        let mut projects: Vec<gpui::AnyElement> = Vec::new();
        for project in self
            .catalog
            .projects
            .iter()
            .filter(|project| !self.is_chat_workspace(project))
        {
            let key = rail_project_shortcut_key(project.id);
            projects.push(check_row(
                SharedString::from(format!("rail-more-{key}")),
                self.project_name(project).to_string(),
                pinned.contains(key.as_str()),
                key,
                cx,
            ));
        }
        div()
            .id("rail-more-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.rail.more_open = false;
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .id("rail-more-menu")
                    .role(gpui::Role::Menu)
                    .aria_label("Rail shortcuts")
                    .tab_group()
                    .absolute()
                    .top(px(ui::CHROME_HEIGHT + 8.))
                    .left(px(RAIL_WIDTH + 8.))
                    .w(px(240.))
                    .max_h(px(480.))
                    .overflow_y_scroll()
                    .p_1()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .bg(rgb(palette().overlay))
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(self.settings.value.general.show_studio.then(|| {
                        ui::action_icon(
                            "rail-more-studio",
                            "Studio",
                            Some(ui::central_icon("images-1").into_any_element()),
                            self.rail.active_item == RailItemId::STUDIO,
                            cx.listener(|this, _: &(), _, cx| {
                                this.rail.more_open = false;
                                this.open_rail_studio(cx);
                            }),
                        )
                        .into_any_element()
                    }))
                    .child(section_label("Spaces in the rail"))
                    .children(spaces)
                    .child(section_label("Projects in the rail"))
                    .children(projects.is_empty().then(|| {
                        div()
                            .px_2()
                            .py_1()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("No projects yet")
                    }))
                    .children(projects)
                    .child(div().my(px(4.)).h(px(1.)).mx_2().bg(rgb(palette().border)))
                    .child(
                        ui::action(
                            "rail-customize",
                            "Customize…",
                            Some(Glyph::Sliders),
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.rail.more_open = false;
                                this.rail.customize_open = true;
                                cx.notify();
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe("rail-customize")),
                    ),
            )
            .into_any_element()
    }

    /// Upstream `SidebarCustomizeList` as the rail popover: every rail item
    /// with its visibility checkbox and reorder controls (Home is locked on),
    /// then the pinned shortcuts.
    pub(super) fn rail_customize_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.rail.customize_open {
            return div().into_any_element();
        }
        let (order, hidden, shortcuts) = self.rail_config();
        let studio_available = self.settings.value.general.show_studio;
        let disabled = self.settings.saving;
        let item_rows: Vec<gpui::AnyElement> = order
            .iter()
            .enumerate()
            .filter(|(_, id)| **id != RailOrderableItemId::Studio || studio_available)
            .map(|(index, id)| {
                let id = *id;
                let item = id.item();
                let visible = !hidden.contains(&id);
                let locked = !rail_item_can_hide(id);
                div()
                    .id(SharedString::from(format!("rail-customize-row-{index}")))
                    .px_2()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        self.rail_glyph_button(
                            SharedString::from(format!("rail-customize-visible-{index}")),
                            if visible { "Hide" } else { "Show" },
                            ui::icon(if visible { Glyph::Check } else { Glyph::Close })
                                .into_any_element(),
                            locked || disabled,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.save_setting(
                                    move |settings| {
                                        let mut hidden = normalize_hidden_rail_items(
                                            &settings.general.hidden_rail_items,
                                        );
                                        if visible {
                                            hidden.retain(|entry| *entry != id);
                                        } else if !hidden.contains(&id) {
                                            hidden.push(id);
                                        }
                                        settings.general.hidden_rail_items = hidden
                                            .iter()
                                            .map(|entry| entry.name().to_owned())
                                            .collect();
                                    },
                                    cx,
                                );
                            }),
                        )
                        .size(px(22.)),
                    )
                    .child(ui::icon(Glyph::Pin).size(px(14.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .child(rail_item_label(item)),
                    )
                    .child(
                        ui::button(
                            SharedString::from(format!("rail-customize-up-{index}")),
                            "↑",
                            false,
                        )
                        .aria_label(format!("Move {} up", rail_item_label(item)))
                        .when(index == 0 || disabled, |button| button.opacity(0.4))
                        .on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                this.move_rail_item(id, true, cx)
                            },
                        )),
                    )
                    .child(
                        ui::button(
                            SharedString::from(format!("rail-customize-down-{index}")),
                            "↓",
                            false,
                        )
                        .aria_label(format!("Move {} down", rail_item_label(item)))
                        .when(index + 1 == order.len() || disabled, |button| {
                            button.opacity(0.4)
                        })
                        .on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                this.move_rail_item(id, false, cx)
                            },
                        )),
                    )
                    .into_any_element()
            })
            .collect();
        let shortcut_rows: Vec<gpui::AnyElement> = shortcuts
            .iter()
            .enumerate()
            .map(|(index, shortcut)| {
                let key = shortcut.key().to_owned();
                let up_key = key.clone();
                let down_key = key.clone();
                let label = match shortcut {
                    RailShortcut::Space { space_id, .. } => match space_id {
                        None => "Void".to_owned(),
                        Some(id) => self
                            .organization
                            .value
                            .spaces
                            .iter()
                            .find(|space| &space.id == id)
                            .map(|space| space.name.clone())
                            .unwrap_or_else(|| "Unknown space".to_owned()),
                    },
                    RailShortcut::Project { project_id, .. } => self
                        .catalog
                        .projects
                        .iter()
                        .find(|project| &project.id == project_id)
                        .map(|project| self.project_name(project).to_string())
                        .unwrap_or_else(|| "Unavailable project".to_owned()),
                };
                let remove_key = key.clone();
                div()
                    .id(SharedString::from(format!("rail-shortcut-row-{index}")))
                    .px_2()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        self.rail_glyph_button(
                            SharedString::from(format!("rail-shortcut-off-{index}")),
                            "Remove from rail",
                            ui::icon(Glyph::Check).into_any_element(),
                            disabled,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.toggle_rail_shortcut(remove_key.clone(), cx)
                            }),
                        )
                        .size(px(22.)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .child(label.clone()),
                    )
                    .child(
                        ui::button(
                            SharedString::from(format!("rail-shortcut-up-{index}")),
                            "↑",
                            false,
                        )
                        .aria_label(format!("Move {label} up"))
                        .when(index == 0 || disabled, |button| button.opacity(0.4))
                        .on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                this.move_rail_shortcut(&up_key, true, cx)
                            },
                        )),
                    )
                    .child(
                        ui::button(
                            SharedString::from(format!("rail-shortcut-down-{index}")),
                            "↓",
                            false,
                        )
                        .aria_label(format!("Move {label} down"))
                        .when(index + 1 == shortcuts.len() || disabled, |button| {
                            button.opacity(0.4)
                        })
                        .on_click(cx.listener(
                            move |this, _: &gpui::ClickEvent, _, cx| {
                                this.move_rail_shortcut(&down_key, false, cx)
                            },
                        )),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .id("rail-customize-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.rail.customize_open = false;
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .id("rail-customize-popover")
                    .absolute()
                    .top(px(ui::CHROME_HEIGHT + 8.))
                    .left(px(RAIL_WIDTH + 8.))
                    .w(px(260.))
                    .max_h(px(480.))
                    .overflow_y_scroll()
                    .p_1()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .bg(rgb(palette().overlay))
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child("Customize"),
                            )
                            .child(ui::button("rail-customize-done", "Done", false).on_click(
                                cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.rail.customize_open = false;
                                    cx.notify();
                                }),
                            )),
                    )
                    .children(item_rows)
                    .children((!shortcut_rows.is_empty()).then(|| {
                        div()
                            .px_2()
                            .pt_2()
                            .pb_1()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child("Shortcuts")
                    }))
                    .children(shortcut_rows),
            )
            .into_any_element()
    }

    /// `railItemOrder` swap (upstream `arrayMove` on the saved order).
    fn move_rail_item(&mut self, id: RailOrderableItemId, backwards: bool, cx: &mut Context<Self>) {
        self.save_setting(
            move |settings| {
                let mut order = normalize_rail_item_order(&settings.general.rail_item_order);
                let Some(index) = order.iter().position(|entry| *entry == id) else {
                    return;
                };
                let target = if backwards {
                    index.checked_sub(1)
                } else {
                    index.checked_add(1).filter(|target| *target < order.len())
                };
                let Some(target) = target else { return };
                order.swap(index, target);
                settings.general.rail_item_order =
                    order.iter().map(|entry| entry.name().to_owned()).collect();
            },
            cx,
        );
    }

    fn move_rail_shortcut(&mut self, key: &str, backwards: bool, cx: &mut Context<Self>) {
        let key = key.to_owned();
        self.save_setting(
            move |settings| {
                let mut keys = settings.general.rail_shortcuts.clone();
                let Some(index) = keys.iter().position(|entry| *entry == key) else {
                    return;
                };
                let target = if backwards {
                    index.checked_sub(1)
                } else {
                    index.checked_add(1).filter(|target| *target < keys.len())
                };
                let Some(target) = target else { return };
                keys.swap(index, target);
                settings.general.rail_shortcuts = keys;
            },
            cx,
        );
    }
}

/// `--app-rail-shell-tone`: the rail strip sits a shade off the sidebar
/// surface — lightened in dark themes, darkened in light.
fn rail_shell_tone() -> u32 {
    let palette = palette();
    mix_color(
        palette.sidebar,
        palette.text,
        if palette.dark { 0.05 } else { 0.03 },
    )
}
/// `--app-rail-inset-border`: the hairline between the rail's items and
/// shortcuts.
fn rail_inset_border() -> gpui::Rgba {
    rgba((palette().text << 8) | if palette().dark { 0x17 } else { 0x1a })
}

fn mix_color(a: u32, b: u32, amount: f32) -> u32 {
    let channel = |shift| {
        let a = ((a >> shift) & 0xff_u32) as f32;
        let b = ((b >> shift) & 0xff_u32) as f32;
        (a + (b - a) * amount).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}
