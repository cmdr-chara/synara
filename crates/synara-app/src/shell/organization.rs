//! Space tabs and project/thread organization use persisted metadata, not a
//! second catalog. Switching a Space only filters the sidebar, preserving work.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;
mod dialog;
use dialog::{DialogEvent, OrganizationDialog};

pub(super) enum OrganizationReply {
    Loaded(Result<WorkspaceOrganization, String>),
    Saved(Result<WorkspaceOrganization, String>),
}
pub(super) struct OrganizationState {
    pub value: WorkspaceOrganization,
    pub loaded: bool,
    loading: bool,
    pub saving: bool,
    pub dialog: Option<Entity<OrganizationDialog>>,
    subscription: Option<Subscription>,
    previous_focus: Option<FocusHandle>,
    restore_focus: bool,
    focus_space: Option<Option<String>>,
    tab_focus: BTreeMap<Option<String>, FocusHandle>,
}
impl OrganizationState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self { value: WorkspaceOrganization::default(), loaded: false, loading: false,
            saving: false, dialog: None, subscription: None, previous_focus: None,
            restore_focus: false, focus_space: None,
            tab_focus: BTreeMap::from([(None, cx.focus_handle())]) }
    }
}
fn symbol_glyph(symbol: SpaceSymbol) -> Glyph {
    match symbol { SpaceSymbol::Folder => Glyph::Folder, SpaceSymbol::Star => Glyph::Star,
        SpaceSymbol::Code => Glyph::Terminal, SpaceSymbol::Globe => Glyph::Browser }
}
impl Shell {
    pub(super) fn load_organization(&mut self) {
        if self.organization.loading || self.organization.saving { return; }
        self.organization.loading = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Organization(Box::new(OrganizationReply::Loaded(
                workspace.organization().await.map_err(|error| error.to_string())
            ))))
        });
    }
    pub(super) fn edit_organization(&mut self, edit: OrganizationEdit, cx: &mut Context<Self>) {
        if !self.organization.loaded || self.organization.saving || self.close != CloseState::Open {
            if let Some(dialog) = &self.organization.dialog {
                dialog.update(cx, |dialog, cx| dialog.saved(Err("Wait for the current save or close operation to finish.".into()), cx));
            }
            return;
        }
        self.organization.saving = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Organization(Box::new(OrganizationReply::Saved(
                workspace.edit_organization(edit).await.map_err(|error| error.to_string())
            ))))
        });
        cx.notify();
    }
    pub(super) fn organization_reply(&mut self, reply: OrganizationReply, cx: &mut Context<Self>) {
        let (saved, result) = match reply {
            OrganizationReply::Loaded(result) => { self.organization.loading = false; (false, result) },
            OrganizationReply::Saved(result) => { self.organization.saving = false; (true, result) },
        };
        match &result {
            Ok(value) => {
                if !self.organization.loaded || value.revision >= self.organization.value.revision {
                    self.organization.value = value.clone();
                    self.organization.loaded = true;
                    self.navigation.project_page = 0;
                    self.organization.tab_focus.retain(|id, _| id.is_none() || value.spaces.iter().any(|space| Some(&space.id) == id.as_ref()));
                    for space in &value.spaces {
                        self.organization.tab_focus.entry(Some(space.id.clone())).or_insert_with(|| cx.focus_handle());
                    }
                }
            }
            Err(error) => {
                self.notice = Some(format!("Spaces could not be {}: {error}. Existing projects, conversations and saved organization are unchanged.", if saved { "saved" } else { "loaded" }));
            }
        }
        if saved && let Some(dialog) = &self.organization.dialog {
            dialog.update(cx, |dialog, cx| dialog.saved(result, cx));
        }
        cx.notify();
    }
    pub(super) fn project_in_active_space(&self, project: ProjectId) -> bool {
        !self.organization.loaded || self.organization.value.contains_project(project)
    }
    pub(super) fn pinned_project(&self, project: ProjectId) -> bool {
        self.organization.value.pinned_projects.contains(&project)
    }
    pub(super) fn pinned_thread(&self, task: TaskId) -> bool {
        self.organization.value.pinned_threads.contains(&task)
    }
    pub(super) fn open_organization(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.organization.dialog.is_some() || self.kanban.dialog.is_some() || self.close != CloseState::Open { return; }
        if !self.organization.loaded {
            self.load_organization();
            self.notice = Some("Loading saved Spaces. Reopen the manager when loading finishes. Unreadable data is never replaced automatically.".into());
            cx.notify(); return;
        }
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.settings.popup = None;
        self.navigation.menu_open = false;
        self.focus_composer = false;
        let projects = self.catalog.projects.iter().filter(|project| !self.is_chat_workspace(project)).cloned().collect();
        let dialog = cx.new(|cx| OrganizationDialog::new(self.organization.value.clone(), projects, cx));
        self.organization.subscription = Some(cx.subscribe(&dialog, |this, _, event, cx| {
            match event {
                DialogEvent::Edit(edit) => this.edit_organization(edit.clone(), cx),
                DialogEvent::Dismiss => {
                    this.organization.dialog = None;
                    this.organization.restore_focus = true;
                }
            }
            cx.notify();
        }));
        self.organization.previous_focus = window.focused(cx);
        self.organization.dialog = Some(dialog);
        cx.notify();
    }
    pub(super) fn restore_organization_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.organization.restore_focus {
            self.organization.restore_focus = false;
            if let Some(focus) = self.organization.previous_focus.take() { window.focus(&focus, cx); }
        }
        if !self.organization.saving && let Some(id) = self.organization.focus_space.take()
            && let Some(focus) = self.organization.tab_focus.get(&id)
        { window.focus(focus, cx); }
    }
    fn select_space(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.organization.saving || !self.organization.loaded { return; }
        self.organization.focus_space = Some(id.clone());
        self.edit_organization(OrganizationEdit::Select(id), cx);
    }
    fn cycle_space(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let ids: Vec<_> = std::iter::once(None).chain(self.organization.value.spaces.iter().map(|space| Some(space.id.clone()))).collect();
        let index = ids.iter().position(|id| id == &self.organization.value.active).unwrap_or(0);
        let next = if backwards { (index + ids.len() - 1) % ids.len() } else { (index + 1) % ids.len() };
        self.select_space(ids[next].clone(), cx);
    }
    pub(super) fn organization_shortcut(&mut self, event: &gpui::KeyDownEvent, window: &Window, cx: &mut Context<Self>) -> bool {
        let modifiers = event.keystroke.modifiers;
        if self.navigation.studio || self.organization.dialog.is_some() || self.organization.saving
            || event.prefer_character_input || event.is_held || self.close != CloseState::Open
            || self.controls.is_open() || self.environment.menu_open() || self.chat_tools.menu_open()
            || self.settings.popup.is_some() || self.navigation.menu_open
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).focus_handle(cx).is_focused(window)
            || self.terminal_view.read(cx).focus_handle(cx).is_focused(window)
            || !modifiers.alt || !(modifiers.control || modifiers.platform) || modifiers.shift
        { return false; }
        match event.keystroke.key.as_str() {
            "left" | "right" => self.cycle_space(event.keystroke.key == "left", cx),
            "1" => self.select_space(None, cx),
            value => {
                let Some(index) = value.parse::<usize>().ok().filter(|index| (2..=9).contains(index)) else { return false; };
                let Some(id) = self.organization.value.spaces.get(index - 2).map(|space| space.id.clone()) else { return false; };
                self.select_space(Some(id), cx);
            }
        }
        true
    }
    pub(super) fn space_strip(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut tabs = vec![(None, "Void".to_owned(), Glyph::Folder)];
        tabs.extend(self.organization.value.spaces.iter().map(|space| (Some(space.id.clone()), space.name.clone(), symbol_glyph(space.symbol))));
        let disabled = !self.organization.loaded || self.organization.saving;
        div().id("space-strip").relative().child(ui::layout_probe("space-strip"))
            .px_3().py_1().h(px(36.)).flex().items_center().gap_1().min_w_0().flex_shrink_0()
            .child(div().id("space-tabs").role(gpui::Role::TabList).aria_label("Spaces")
                .flex().items_center().gap_1().flex_1().min_w_0().overflow_x_scroll()
                .children(tabs.into_iter().enumerate().map(|(index, (id, name, glyph))| {
                    let selected = self.organization.value.active == id;
                    let waiting = self.catalog.tasks.iter().any(|task| task.scope == TaskScope::Project
                        && self.organization.value.space_for(task.project_id) == id.as_deref()
                        && task.state == TaskState::Waiting);
                    let running = self.catalog.tasks.iter().any(|task| task.scope == TaskScope::Project
                        && self.organization.value.space_for(task.project_id) == id.as_deref()
                        && (task.state == TaskState::Running || self.busy.contains(&task.id)));
                    let tooltip = format!("{name}{}", if waiting { " · Needs attention" } else if running { " · Working" } else { "" });
                    let choose = id.clone(); let move_id = id.clone();
                    ui::button_shell(SharedString::from(format!("space-tab-{index}")), name, selected)
                        .role(gpui::Role::Tab).aria_selected(selected).aria_label(tooltip.clone())
                        .aria_description("Left/Right switch Spaces. Alt+Left/Right reorder a named Space.")
                        .tab_stop(selected).when_some(self.organization.tab_focus.get(&id).cloned(), |el, focus| el.track_focus(&focus))
                        .size(px(26.)).p_0().flex_shrink_0().flex().items_center().justify_center()
                        .when(selected, |el| el.border_1().border_color(rgb(palette().border)))
                        .when(disabled, |el| el.opacity(0.5))
                        .relative().child(ui::layout_probe_slot("space-tab", index))
                        .child(ui::icon(glyph).size(px(15.)))
                        .children((waiting || running).then(|| div().absolute().right_0().top_0().size(px(5.)).rounded_full().bg(rgb(if waiting { palette().error } else { palette().focus }))))
                        .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(tooltip.clone().into())).into())
                        .on_click(cx.listener(move |this, _, _, cx| this.select_space(choose.clone(), cx)))
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            let modifiers = event.keystroke.modifiers;
                            if modifiers.control || modifiers.platform || modifiers.shift || event.prefer_character_input { return; }
                            let backwards = event.keystroke.key == "left";
                            if !matches!(event.keystroke.key.as_str(), "left" | "right") { return; }
                            if modifiers.alt {
                                if let Some(id) = &move_id { this.edit_organization(OrganizationEdit::Move { id: id.clone(), backwards }, cx); }
                            } else { this.cycle_space(backwards, cx); }
                            cx.stop_propagation();
                        }))
                })))
            .child(ui::chrome_button("spaces-manage", "Manage Spaces and projects", Glyph::Plus, self.organization.saving,
                cx.listener(|this, _: &(), window, cx| this.open_organization(window, cx))).size(px(26.)))
            .into_any_element()
    }
    pub(super) fn thread_pin_button(&self, task: &Task, cx: &mut Context<Self>) -> gpui::AnyElement {
        let id = task.id;
        let pinned = self.pinned_thread(id);
        let disabled = !self.organization.loaded || self.organization.saving;
        ui::button_shell(SharedString::from(format!("pin-thread-{id}")), "Pin thread", false)
            .aria_label(if pinned { "Unpin thread" } else { "Pin thread" })
            .size(px(22.)).p_0().bg(gpui::rgba(0)).flex().items_center().justify_center()
            .when(pinned, |el| el.text_color(rgb(palette().focus)))
            .when(!pinned, |el| el.opacity(0.45))
            .when(disabled, |el| el.opacity(0.25))
            .child(ui::icon(Glyph::Pin).size(px(13.)))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if !disabled { this.edit_organization(OrganizationEdit::PinThread { task: id, pinned: !pinned }, cx); }
            })).into_any_element()
    }
}
