use super::*;

fn label(text: &'static str) -> gpui::Div {
    div().text_size(px(12.)).text_color(rgb(palette().muted)).mt_4().mb_2().child(text)
}
impl Shell {
    pub(in crate::shell) fn hub_sidebar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.hubs.query.read(cx).text().trim().to_lowercase();
        let rows: Vec<_> = self.hubs.rows.iter().filter(|hub|
            hub.profile.archived == self.hubs.show_archived && hub.profile.name.to_lowercase().contains(&query)).collect();
        div().id("hub-navigation").role(gpui::Role::Navigation).aria_label("Hubs").tab_group()
            .w(px(ui::SIDEBAR_WIDTH)).h_full().min_h_0().flex().flex_col().bg(ui::surface(palette().sidebar))
            .child(div().px_2().py_2().flex().items_center().gap_1()
                .child(ui::action("hubs-back", "Synara", Some(Glyph::Back), false,
                    cx.listener(|this, _: &(), _, cx| this.switch_mode(false,cx))).flex_1())
                .child(ui::chrome_button("hub-create", "New Hub", Glyph::Plus, self.hubs.saving,
                    cx.listener(|this, _: &(), _, cx| this.edit_hub(true,cx)))))
            .child(div().px_3().py_2().child(self.hubs.query.clone()))
            .children(self.hubs.selected.map(|id| ui::action("hub-sidebar-tasks", "Tasks", Some(Glyph::Kanban), self.panel == Panel::Kanban,
                cx.listener(move |this, _: &(), _, cx| this.open_hub_tasks(id, cx))).mx_2().rounded_none()))
            .child(div().id("hub-list").flex_1().min_h_0().overflow_y_scroll().px_2().py_2()
                .children(rows.iter().take(200).enumerate().map(|(index,hub)| {
                    let id = hub.profile.project;
                    let active = self.hubs.selected == Some(id);
                    let mut tasks: Vec<_> = self.catalog.tasks.iter().filter(|task|
                        task.project_id == id && task.scope == TaskScope::Studio && task.state != TaskState::Archived).collect();
                    tasks.sort_by_key(|task| (!self.pinned_thread(task.id),std::cmp::Reverse(task.updated_at_ms)));
                    div().flex().flex_col().mb_2()
                        .child(ui::action(("hub-row",index),hub.profile.name.clone(),Some(Glyph::Folders),active && self.panel == Panel::Hubs,
                            cx.listener(move |this, _: &(), _, cx| this.open_hub(id,cx))).w_full()
                            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(hub.threads.to_string())))
                        .children(active.then(|| div().pl_3().flex().flex_col().children(tasks.iter().take(32).enumerate().map(|(slot,task)| {
                            let task_id = task.id;
                            ui::action(("hub-thread",slot),task.title.clone(),Some(self.agent_glyph(&task.agent_id)),self.selected == Some(task_id) && (self.panel == Panel::Conversation || self.dock_open()),
                                cx.listener(move |this, _: &(), _, cx| { if this.select_task(task_id,cx) { this.show_conversation(cx); } }))
                                .w_full().h(px(ui::row_height())).text_size(px(12.))
                                .children((self.busy.contains(&task_id)||self.connecting.contains(&task_id)).then(||
                                    div().size(px(4.)).rounded_full().bg(rgb(palette().focus))))
                        }))))
                }))
                .children(self.hubs.loading.then(||div().p_3().text_color(rgb(palette().muted)).child("Loading Hubs...")))
                .children((self.hubs.loaded && rows.is_empty()).then(||div().p_3().text_size(px(13.)).text_color(rgb(palette().muted)).child("No matching Hubs. Normal chats need no Hub.")))
                .children((rows.len() > 200).then(||div().p_3().text_size(px(12.)).child("Showing 200 Hubs. Narrow the search to find another."))))
            .child(div().p_2().border_t_1().border_color(rgb(palette().border)).flex().flex_col()
                .child(ui::action("hub-find-thread","Find any thread",Some(Glyph::Search),false,
                    cx.listener(|this, _: &(), window, cx| this.open_thread_finder(window,cx))))
                .child(ui::action("hub-archive-filter",if self.hubs.show_archived {"Show active Hubs"} else {"Archived Hubs"},Some(Glyph::Archive),false,
                    cx.listener(|this, _: &(), _, cx| { this.hubs.show_archived = !this.hubs.show_archived; cx.notify(); })))
                .child(ui::action("hub-settings","Settings",Some(Glyph::Settings),false,
                    cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Settings,cx)))))
            .into_any_element()
    }
    pub(in crate::shell) fn hub_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut page = div().flex().flex_col().gap_2().w_full().max_w(px(960.)).mx_auto();
        if self.hubs.editing {
            let creating = self.hubs.original.is_none();
            page = page.child(div().text_size(px(24.)).child(if creating {"New Hub"} else {"Hub context"}))
                .child(div().text_size(px(13.)).text_color(rgb(palette().muted)).child(if creating {
                    "A place for related work. A repository is optional."
                } else {"Shared instructions and knowledge seed new visible drafts. Conversations and approvals stay separate."}))
                .child(label("Name")).child(self.hubs.name.clone());
            if creating {
                page = page.child(label("Working folder"))
                    .child(div().text_size(px(13.)).child(self.hubs.folder.as_ref().map_or("Synara-managed folder (no repository needed)".into(),|p|p.display().to_string())))
                    .child(div().flex().flex_wrap().gap_2()
                        .child(ui::action("hub-choose-folder","Choose folder...",Some(Glyph::Folder),false,cx.listener(|this, _: &(), _, cx| this.pick_hub_folder(cx))))
                        .child(ui::action("hub-managed-folder","Use managed folder",None,false,cx.listener(|this, _: &(), _, cx| {
                            if !this.hubs.creating && !this.hubs.picker { this.hubs.folder = None; cx.notify(); }
                        }))));
            } else {
                page = page.child(label("Description")).child(self.hubs.description.clone())
                    .child(label("Instructions")).child(div().h(px(160.)).flex().flex_col().child(self.hubs.instructions.clone()))
                    .child(label("Shared knowledge")).child(div().h(px(180.)).flex().flex_col().child(self.hubs.memory.clone()))
                    .child(ui::action("hub-context-default",if self.hubs.include {"Include context in new thread drafts: On"} else {"Include context in new thread drafts: Off"},None,self.hubs.include,
                        cx.listener(|this, _: &(), _, cx| { this.hubs.include = !this.hubs.include; cx.notify(); })))
                    .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("This is curated knowledge, not automatic memory. Existing threads only receive updates when you explicitly add them to a draft."));
            }
            page = page.child(div().mt_3().pt_3().border_t_1().border_color(rgb(palette().border)).flex().flex_wrap().gap_3()
                .child(ui::action("hub-save",if self.hubs.saving || self.hubs.creating {"Saving..."} else if creating {"Create Hub"} else {"Save context"},Some(Glyph::Check),false,
                    cx.listener(|this, _: &(), _, cx| this.save_hub_editor(cx))))
                .child(ui::action("hub-discard","Discard edits",None,false,cx.listener(|this, _: &(), _, cx| {
                    if !this.hubs.saving && !this.hubs.creating && !this.hubs.picker {
                        this.hubs.editing = false; this.hubs.original = None; this.load_hubs(); cx.notify();
                    }
                })))
                .child(ui::action("hub-copy-context","Copy edits",Some(Glyph::Copy),false,cx.listener(|this, _: &(), _, cx| {
                    let text = [this.hubs.name.read(cx).text(),this.hubs.description.read(cx).text(),
                        this.hubs.instructions.read(cx).text(),this.hubs.memory.read(cx).text()].join("\n\n");
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                }))));
        } else if let Some(profile) = self.hub_profile() {
            let id = profile.project;
            let mut threads: Vec<_> = self.catalog.tasks.iter().filter(|task|
                task.project_id == id && task.scope == TaskScope::Studio && task.state != TaskState::Archived).collect();
            threads.sort_by_key(|task| (task.id != profile.main_task,std::cmp::Reverse(task.updated_at_ms)));
            let running = threads.iter().filter(|task|self.busy.contains(&task.id)||self.connecting.contains(&task.id)).count();
            page = page.child(div().flex().flex_wrap().items_center().gap_3()
                .child(div().flex_1().text_size(px(26.)).child(profile.name.clone()))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("{} threads · {running} active",threads.len()))))
                .children((!profile.description.is_empty()).then(||div().text_color(rgb(palette().muted)).child(profile.description.clone())))
                .child(div().mt_4().pb_3().border_b_1().border_color(rgb(palette().border)).flex().flex_wrap().gap_2()
                    .child(ui::action("hub-new-thread","New thread",Some(Glyph::Compose),false,cx.listener(|this, _: &(), _, cx| this.new_hub_thread(cx))))
                    .child(ui::action("hub-open-tasks", "Tasks", Some(Glyph::Kanban), false,
                        cx.listener(move |this, _: &(), _, cx| this.open_hub_tasks(id, cx))))
                    .child(ui::action("hub-open-library","Library",Some(Glyph::Files),false,cx.listener(|this, _: &(), _, cx| this.open_studio_outputs(cx))))
                    .child(ui::action("hub-edit-context","Context",Some(Glyph::Notebook),false,cx.listener(|this, _: &(), _, cx| this.edit_hub(false,cx))))
                    .child(ui::action("hub-refresh","Refresh",Some(Glyph::Restore),false,cx.listener(|this, _: &(), _, cx| {this.load_hubs();cx.notify();}))))
                .child(label("Threads"))
                .children(threads.iter().take(200).enumerate().map(|(index,task)| {
                    let id = task.id;
                    ui::action(("hub-home-thread",index),task.title.clone(),Some(self.agent_glyph(&task.agent_id)),false,
                        cx.listener(move |this, _: &(), _, cx| { if this.select_task(id,cx) {this.show_conversation(cx);} }))
                        .w_full().h(px(ui::row_height() + 6.)).rounded_none().bg(gpui::rgba(0)).border_b_1().border_color(rgb(palette().border))
                        .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("{:?}",task.state)))
                }))
                .children((threads.len() > 200).then(|| ui::action("hub-find-more","Find more threads",Some(Glyph::Search),false,
                    cx.listener(|this, _: &(), window, cx| this.open_thread_finder(window,cx)))))
                .child(label("Shared context"))
                .child(div().text_size(px(13.)).text_color(rgb(palette().muted)).child(format!("Instructions: {} bytes · Knowledge: {} bytes · Revision {}",profile.instructions.len(),profile.memory.len(),profile.revision)))
                .child(ui::action("hub-use-context","Add shared context to current draft",Some(Glyph::Compose),false,cx.listener(|this, _: &(), _, cx| this.use_hub_context(cx))))
                .child(div().mt_6().pt_3().border_t_1().border_color(rgb(palette().border)).flex().flex_wrap().items_center().gap_2()
                    .child(ui::action("hub-archive",if profile.archived {"Restore Hub"} else {"Archive Hub"},Some(Glyph::Archive),false,cx.listener(|this, _: &(), _, cx| this.archive_hub(cx))))
                    .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Archiving keeps files, conversations and running work. It does not delete or stop anything.")));
        } else {
            page = page.child(div().mt_6().text_size(px(26.)).child("Hubs"))
                .child(div().text_color(rgb(palette().muted)).child("Optional shared context for related work. Your normal Synara chats stay as they are."))
                .child(div().mt_4().flex().gap_3()
                    .child(ui::action("hub-first-create","New Hub",Some(Glyph::Plus),false,cx.listener(|this, _: &(), _, cx| this.edit_hub(true,cx))))
                    .child(ui::action("hub-reload","Reload Hubs",Some(Glyph::Restore),false,cx.listener(|this, _: &(), _, cx| {this.load_hubs();cx.notify();}))));
        }
        div().id("hub-page").flex_1().min_w_0().min_h_0().overflow_y_scroll().p_6().child(page).into_any_element()
    }
}
