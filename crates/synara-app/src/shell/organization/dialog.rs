//! Native Space manager. Emits explicit organization edits, never filesystem work.
use super::*;
use crate::ui::menu::{Choice, ChoiceEvent, ChoiceMenu};
use gpui::{EventEmitter, FocusHandle, KeyDownEvent};

pub(super) enum DialogEvent {
    Edit(OrganizationEdit),
    Dismiss,
}
struct Editor {
    id: Option<String>,
    original: String,
    symbol: SpaceSymbol,
    original_symbol: SpaceSymbol,
}
struct Assignment {
    view: Entity<ChoiceMenu>,
    _subscription: Subscription,
}
pub(super) struct OrganizationDialog {
    value: WorkspaceOrganization,
    projects: Vec<Project>,
    name: Entity<TextEntry>,
    query: Entity<TextEntry>,
    focus: FocusHandle,
    editor: Option<Editor>,
    assignment: Option<Assignment>,
    delete: Option<String>,
    discard: bool,
    saving: bool,
    saving_editor: bool,
    error: Option<String>,
    focus_name: bool,
    needs_focus: bool,
    shown: usize,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<DialogEvent> for OrganizationDialog {}
impl OrganizationDialog {
    pub fn new(value: WorkspaceOrganization, projects: Vec<Project>, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| TextEntry::new("Space name", EntryMode::SingleLine, 36., cx));
        let query = cx.new(|cx| TextEntry::new("Find a project...", EntryMode::SingleLine, 32., cx).with_leading_icon(Glyph::Search));
        let subscriptions = vec![
            cx.subscribe(&name, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) { this.save_editor(cx); }
                cx.notify();
            }),
            cx.subscribe(&query, |this, _, _, cx| { this.shown = 100; cx.notify(); }),
        ];
        Self { value, projects, name, query, focus: cx.focus_handle(), editor: None,
            assignment: None, delete: None, discard: false, saving: false,
            saving_editor: false, error: None, focus_name: false, needs_focus: true,
            shown: 100, _subscriptions: subscriptions }
    }
    pub fn saved(&mut self, result: Result<WorkspaceOrganization, String>, cx: &mut Context<Self>) {
        self.saving = false;
        match result {
            Ok(value) => {
                self.value = value;
                if self.saving_editor { self.editor = None; self.needs_focus = true; }
                self.delete = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        self.saving_editor = false;
        cx.notify();
    }
    fn edit(&mut self, edit: OrganizationEdit, cx: &mut Context<Self>) {
        if self.saving { return; }
        self.saving = true;
        self.error = None;
        cx.emit(DialogEvent::Edit(edit));
        cx.notify();
    }
    fn dirty(&self, cx: &Context<Self>) -> bool {
        self.editor.as_ref().is_some_and(|editor| self.name.read(cx).text() != editor.original
            || editor.symbol != editor.original_symbol)
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.saving { return; }
        if self.dirty(cx) { self.discard = true; cx.notify(); }
        else { cx.emit(DialogEvent::Dismiss); }
    }
    fn begin_edit(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.saving { return; }
        if self.dirty(cx) {
            self.error = Some("Save or cancel the current Space edit first.".into());
            cx.notify(); return;
        }
        let space = id.as_ref().and_then(|id| self.value.spaces.iter().find(|space| &space.id == id));
        let name = space.map_or(String::new(), |space| space.name.clone());
        let symbol = space.map_or(SpaceSymbol::Folder, |space| space.symbol);
        self.name.update(cx, |entry, cx| entry.set_text(name.clone(), cx));
        self.editor = Some(Editor { id, original: name, symbol, original_symbol: symbol });
        self.focus_name = true;
        self.error = None;
        cx.notify();
    }
    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else { return; };
        if self.saving { return; }
        let name = self.name.read(cx).text().trim().to_owned();
        if name.is_empty() || name.chars().count() > 40 || name.chars().any(char::is_control) {
            self.error = Some("Enter a Space name of 1 to 40 characters.".into());
            cx.notify(); return;
        }
        let edit = match &editor.id {
            Some(id) => OrganizationEdit::Rename { id: id.clone(), name, symbol: editor.symbol },
            None => OrganizationEdit::Create { name, symbol: editor.symbol },
        };
        self.saving_editor = true;
        self.edit(edit, cx);
    }
    fn assign(&mut self, project: ProjectId, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving { return; }
        let destinations: Vec<_> = std::iter::once(None).chain(self.value.spaces.iter().map(|space| Some(space.id.clone()))).collect();
        let mut choices = vec![Choice { label: "Void".into(), detail: "Unassigned projects".into(),
            selected: self.value.space_for(project).is_none(), icon: Some(Glyph::Folder), ..Default::default() }];
        choices.extend(self.value.spaces.iter().map(|space| Choice {
            label: space.name.clone(), selected: self.value.space_for(project) == Some(space.id.as_str()),
            icon: Some(symbol_glyph(space.symbol)), ..Default::default()
        }));
        let view = cx.new(|cx| ChoiceMenu::new("Move project to Space".into(), choices, cx));
        let subscription = cx.subscribe(&view, move |this, _, event, cx| {
            this.assignment = None;
            this.needs_focus = true;
            if let ChoiceEvent::Selected(index) = event
                && let Some(space) = destinations.get(*index)
            { this.edit(OrganizationEdit::Assign { project, space: space.clone() }, cx); }
            cx.notify();
        });
        window.focus(&view.read(cx).focus_handle(cx), cx);
        self.assignment = Some(Assignment { view, _subscription: subscription });
        cx.notify();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input || self.name.read(cx).is_composing() || self.query.read(cx).is_composing() { return; }
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key == "escape" {
            if self.assignment.take().is_some() { self.needs_focus = true; }
            else if self.delete.take().is_some() || self.discard { self.discard = false; }
            else { self.dismiss(cx); }
            cx.notify(); cx.stop_propagation();
        } else if event.keystroke.key == "tab" && !modifiers.control && !modifiers.platform && !modifiers.alt {
            if modifiers.shift { window.focus_prev(cx); } else { window.focus_next(cx); }
            cx.stop_propagation();
        }
    }
}
impl gpui::Render for OrganizationDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.assignment.is_none() {
            if self.focus_name {
                window.focus(&self.name.read(cx).focus_handle(cx), cx);
                self.focus_name = false;
                self.needs_focus = false;
            } else if self.needs_focus {
                window.focus(&self.focus, cx);
                self.needs_focus = false;
            }
        }
        let busy = self.saving;
        let query = self.query.read(cx).text().trim().to_lowercase();
        let projects: Vec<_> = self.projects.iter().filter(|project| query.is_empty() || project.name.to_lowercase().contains(&query)).take(self.shown).cloned().collect();
        let editor = self.editor.as_ref().map(|editor| {
            let symbol = editor.symbol;
            div().p_3().rounded_md().border_1().border_color(rgb(palette().border)).flex().flex_col().gap_2()
                .child(div().text_size(px(13.)).child(if editor.id.is_some() { "Edit Space" } else { "New Space" }))
                .child(div().relative().child(ui::layout_probe("space-name-input")).child(self.name.clone()))
                .child(div().flex().gap_2().children(SpaceSymbol::ALL.into_iter().enumerate().map(|(index, candidate)| {
                    ui::button_shell(SharedString::from(format!("space-symbol-{index}")), "Space icon", symbol == candidate)
                        .size(px(30.)).p_0().flex().items_center().justify_center()
                        .aria_label(format!("{:?} Space icon", candidate))
                        .child(ui::icon(symbol_glyph(candidate)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.saving && let Some(editor) = &mut this.editor { editor.symbol = candidate; cx.notify(); }
                        }))
                })).child(div().flex_1())
                    .child(ui::button("space-edit-cancel", "Cancel edit", false).on_click(cx.listener(|this, _, _, cx| {
                        if !this.saving { this.editor = None; this.error = None; this.needs_focus = true; cx.notify(); }
                    })))
                    .child(ui::button("space-save", if busy { "Saving..." } else { "Save Space" }, true)
                        .relative().child(ui::layout_probe("space-save"))
                        .when(busy, |el| el.opacity(0.4))
                        .on_click(cx.listener(|this, _, _, cx| this.save_editor(cx)))))
        });
        let spaces = self.value.spaces.iter().enumerate().map(|(index, space)| {
            let id = space.id.clone(); let rename = id.clone(); let delete = id.clone();
            let left = id.clone(); let right = id.clone();
            div().id(("managed-space", index)).flex().items_center().gap_2().h(px(38.))
                .child(ui::icon(symbol_glyph(space.symbol)))
                .child(div().flex_1().min_w_0().text_ellipsis().child(space.name.clone()))
                .child(ui::chrome_button("space-left", "Move Space left", Glyph::Back, busy || index == 0,
                    cx.listener(move |this, _: &(), _, cx| this.edit(OrganizationEdit::Move { id: left.clone(), backwards: true }, cx))))
                .child(ui::chrome_button("space-right", "Move Space right", Glyph::Forward, busy || index + 1 == self.value.spaces.len(),
                    cx.listener(move |this, _: &(), _, cx| this.edit(OrganizationEdit::Move { id: right.clone(), backwards: false }, cx))))
                .child(ui::button("space-edit", "Edit", false).relative().child(ui::layout_probe_slot("space-edit", index))
                    .on_click(cx.listener(move |this, _, _, cx| this.begin_edit(Some(rename.clone()), cx))))
                .child(ui::chrome_button("space-delete", "Delete Space, keep its projects", Glyph::Trash, busy,
                    cx.listener(move |this, _: &(), _, cx| { if !this.saving { this.delete = Some(delete.clone()); cx.notify(); } })))
        }).collect::<Vec<_>>();
        let rows = projects.iter().enumerate().map(|(index, project)| {
            let id = project.id;
            let pinned = self.value.pinned_projects.contains(&id);
            let destination = self.value.space_for(id).and_then(|id| self.value.spaces.iter().find(|space| space.id == id)).map_or("Void", |space| space.name.as_str());
            div().id(("managed-project", index)).flex().items_center().gap_2().h(px(38.))
                .child(ui::icon(Glyph::Folder))
                .child(div().flex_1().min_w_0().text_ellipsis().child(project.name.clone()))
                .child(ui::button("project-space", destination.to_owned(), false).text_size(px(12.))
                    .relative().child(ui::layout_probe_slot("project-space", index))
                    .on_click(cx.listener(move |this, _, window, cx| this.assign(id, window, cx))))
                .child(ui::chrome_button("project-pin", if pinned { "Unpin project" } else { "Pin project" }, Glyph::Pin, busy,
                    cx.listener(move |this, _: &(), _, cx| this.edit(OrganizationEdit::PinProject { project: id, pinned: !pinned }, cx)))
                    .when(pinned, |el| el.text_color(rgb(palette().focus))))
        }).collect::<Vec<_>>();
        let confirmation = self.delete.as_ref().map(|id| {
            let id = id.clone();
            div().p_3().rounded_md().bg(rgb(palette().notice_surface)).flex().flex_col().gap_2()
                .child("Delete this Space? Its projects return to Void. Files and conversations are kept.")
                .child(div().flex().justify_end().gap_2()
                    .child(ui::button("space-delete-cancel", "Keep Space", false).on_click(cx.listener(|this, _, _, cx| { this.delete = None; cx.notify(); })))
                    .child(ui::button("space-delete-confirm", "Delete Space", false).relative().child(ui::layout_probe("space-delete-confirm"))
                        .on_click(cx.listener(move |this, _, _, cx| this.edit(OrganizationEdit::Delete { id: id.clone() }, cx)))))
        });
        let modal = div().id("organization-dialog").role(gpui::Role::Dialog).aria_label("Spaces and projects")
            .track_focus(&self.focus).tab_group().tab_stop(true).occlude().relative()
            .w_full().max_w(px(700.)).max_h(window.viewport_size().height - px(40.))
            .flex().flex_col().min_h_0().rounded(px(16.)).border_1().border_color(rgb(palette().border))
            .bg(rgb(palette().overlay)).shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(Self::key))
            .child(ui::layout_probe("organization-dialog"))
            .child(div().p_4().flex().items_center().gap_2()
                .child(div().flex_1().text_size(px(16.)).child("Spaces & projects"))
                .child(ui::chrome_button("organization-close", "Close Space manager", Glyph::Close, busy,
                    cx.listener(|this, _: &(), _, cx| this.dismiss(cx)))))
            .children(self.error.clone().map(|error| div().px_4().py_2().text_color(rgb(palette().error)).child(error)))
            .child(div().id("organization-scroll").px_4().pb_4().min_h_0().overflow_y_scroll().flex().flex_col().gap_3()
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Spaces group projects. They never move folders or change an agent's workspace."))
                .child(div().flex().items_center().gap_2().child(div().flex_1().child("Spaces"))
                    .child(ui::button("space-create", "New Space", false).relative().child(ui::layout_probe("space-create"))
                        .on_click(cx.listener(|this, _, _, cx| this.begin_edit(None, cx)))))
                .children(editor).children(confirmation)
                .child(div().flex().flex_col().child(div().h(px(32.)).text_color(rgb(palette().muted)).child("Void · projects without a Space")).children(spaces))
                .child(div().border_t_1().border_color(rgb(palette().border)).pt_3().child("Projects"))
                .child(self.query.clone()).children(rows)
                .when(projects.is_empty(), |el| el.child(div().text_color(rgb(palette().muted)).child("No projects match. Add a project from the sidebar first.")))
                .when(projects.len() == self.shown, |el| el.child(ui::button("organization-more", "Show more projects", false).on_click(cx.listener(|this, _, _, cx| { this.shown = this.shown.saturating_add(100); cx.notify(); })))))
            .when(self.discard, |el| el.child(div().p_3().bg(rgb(palette().notice_surface)).flex().items_center().gap_2()
                .child(div().flex_1().child("Discard the unfinished Space edit?"))
                .child(ui::button("space-keep-editing", "Keep editing", false).on_click(cx.listener(|this, _, _, cx| { this.discard = false; cx.notify(); })))
                .child(ui::button("space-discard-edit", "Discard and close", false).on_click(cx.listener(|_, _, _, cx| cx.emit(DialogEvent::Dismiss))))));
        div().absolute().inset_0().size_full().p_4().flex().items_center().justify_center().bg(gpui::rgba(0x00000088)).occlude()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| { this.dismiss(cx); cx.stop_propagation(); }))
            .child(modal)
            .children(self.assignment.as_ref().map(|popup| div().absolute().inset_0().flex().items_center().justify_center().occlude()
                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| { this.assignment = None; this.needs_focus = true; cx.notify(); cx.stop_propagation(); }))
                .child(popup.view.clone())))
    }
}
