//! Upstream project look + favicon (`ProjectSidebarIcon`, `EditProjectDialog`):
//! a renderer-local name/appearance per project plus the project's own favicon
//! resolved from its directory. Upstream caches favicon presence per cwd in a
//! module map; here the resolved `gpui::Image` itself is cached per project id.
use super::*;
use crate::ui::central_icon;
use gpui::{AnyElement, FocusHandle};
use std::{path::Path, rc::Rc};
mod dialog;
use dialog::{DialogEvent, EditProjectDialog};

/// Upstream `presentation`: sidebar rows show the folder with a favicon badge;
/// compact/recent rows show the favicon as the primary glyph.
#[derive(Clone, Copy)]
pub(super) enum ProjectGlyphPresentation {
    Badge,
    Favicon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectGlyphSource {
    Emoji,
    CentralIcon,
    Favicon,
    Folder,
    FolderBadge,
}

fn project_glyph_source(
    appearance: Option<&ProjectAppearance>,
    has_favicon: bool,
    presentation: ProjectGlyphPresentation,
) -> ProjectGlyphSource {
    if matches!(appearance, Some(ProjectAppearance::Emoji { .. })) {
        return ProjectGlyphSource::Emoji;
    }
    if matches!(
        appearance,
        Some(ProjectAppearance::Icon { icon, .. }) if icon != DEFAULT_PROJECT_ICON
    ) {
        return ProjectGlyphSource::CentralIcon;
    }
    match presentation {
        ProjectGlyphPresentation::Favicon if has_favicon => ProjectGlyphSource::Favicon,
        ProjectGlyphPresentation::Favicon => ProjectGlyphSource::Folder,
        ProjectGlyphPresentation::Badge => ProjectGlyphSource::FolderBadge,
    }
}

pub(super) struct ProjectUiState {
    pub uis: HashMap<ProjectId, ProjectUi>,
    /// Resolved favicon per project, mirroring upstream's `projectFaviconPresence`
    /// cache — a project directory's favicon does not change between refreshes.
    pub(super) favicons: HashMap<ProjectId, Arc<gpui::Image>>,
    pub dialog: Option<Entity<EditProjectDialog>>,
    subscription: Option<Subscription>,
    previous_focus: Option<FocusHandle>,
    restore_focus: bool,
}
impl ProjectUiState {
    pub fn new() -> Self {
        Self {
            uis: HashMap::new(),
            favicons: HashMap::new(),
            dialog: None,
            subscription: None,
            previous_focus: None,
            restore_focus: false,
        }
    }
}

pub(super) enum ProjectUiReply {
    Loaded {
        uis: HashMap<ProjectId, ProjectUi>,
        favicons: Vec<(ProjectId, Option<Arc<gpui::Image>>)>,
    },
    Saved {
        project: ProjectId,
        ui: ProjectUi,
        result: Result<(), String>,
    },
}

fn favicon_format(path: &Path) -> Option<gpui::ImageFormat> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "svg" => Some(gpui::ImageFormat::Svg),
        "ico" => Some(gpui::ImageFormat::Ico),
        "png" => Some(gpui::ImageFormat::Png),
        "jpg" | "jpeg" => Some(gpui::ImageFormat::Jpeg),
        "webp" => Some(gpui::ImageFormat::Webp),
        "gif" => Some(gpui::ImageFormat::Gif),
        "bmp" => Some(gpui::ImageFormat::Bmp),
        _ => None,
    }
}

/// The emoji drawn as text sized like the upstream `ProjectEmojiGlyph`'s SVG
/// text box, so it matches the line icons' box instead of the UI font size.
fn emoji_glyph(emoji: &str) -> AnyElement {
    div()
        .w(px(16.))
        .h(px(16.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(15.))
        .child(emoji.to_owned())
        .into_any_element()
}

/// Upstream `ProjectSidebarIcon`: emoji wins, then a non-default Central icon,
/// then the folder — tinted when a color is set — with the favicon as a badge
/// overlay or, in `Favicon` rows, as the primary glyph.
pub(super) fn project_glyph(
    appearance: Option<&ProjectAppearance>,
    favicon: Option<&Arc<gpui::Image>>,
    expanded: bool,
    presentation: ProjectGlyphPresentation,
) -> AnyElement {
    let dark = crate::ui::palette().dark;
    let color = match appearance {
        Some(ProjectAppearance::Icon { color, .. }) => {
            color.as_ref().and_then(|color| color.color())
        }
        _ => None,
    };
    let tint = color.map_or_else(|| rgb(crate::ui::palette().muted), |c| rgb(c.rgb(dark)));
    let source = project_glyph_source(appearance, favicon.is_some(), presentation);
    let folder = || {
        central_icon(if expanded {
            "folder-open-front"
        } else {
            "folder-2"
        })
        .text_color(tint)
    };
    match source {
        ProjectGlyphSource::Emoji => {
            if let Some(ProjectAppearance::Emoji { emoji }) = appearance {
                return emoji_glyph(emoji);
            }
        }
        ProjectGlyphSource::CentralIcon => {
            if let Some(ProjectAppearance::Icon { icon, .. }) = appearance {
                return central_icon(icon).text_color(tint).into_any_element();
            }
        }
        ProjectGlyphSource::Favicon => {
            return favicon
                .map(|image| {
                    gpui::img(image.clone())
                        .size(px(16.))
                        .rounded(px(2.))
                        .flex_shrink_0()
                        .into_any_element()
                })
                .unwrap_or_else(|| folder().into_any_element());
        }
        ProjectGlyphSource::Folder => return folder().into_any_element(),
        ProjectGlyphSource::FolderBadge => {}
    }
    div()
        .relative()
        .flex_shrink_0()
        .child(folder())
        .children(favicon.map(|image| {
            gpui::img(image.clone())
                .absolute()
                .right(px(-4.))
                .bottom(px(-4.))
                .size(px(12.))
                .rounded(px(4.))
        }))
        .into_any_element()
}

impl Shell {
    /// The displayed project name: the local override when set (upstream
    /// `projectLocalNames`), else the project's own name.
    pub(super) fn project_name(&self, project: &Project) -> SharedString {
        self.project_ui
            .uis
            .get(&project.id)
            .and_then(|ui| ui.name.clone())
            .unwrap_or_else(|| project.name.clone())
            .into()
    }
    pub(super) fn project_icon(
        &self,
        project: &Project,
        expanded: bool,
        presentation: ProjectGlyphPresentation,
    ) -> AnyElement {
        let ui = self.project_ui.uis.get(&project.id);
        project_glyph(
            ui.and_then(|ui| ui.appearance.as_ref()),
            self.project_ui.favicons.get(&project.id),
            expanded,
            presentation,
        )
    }
    /// Snapshot only the owned project look needed by a virtualized choice
    /// row. The menu invokes this recipe while rendering instead of retaining
    /// an `AnyElement` or borrowing Shell state.
    pub(super) fn project_choice_icon(&self, project: &Project) -> crate::ui::menu::ChoiceIcon {
        let appearance = self
            .project_ui
            .uis
            .get(&project.id)
            .and_then(|ui| ui.appearance.clone());
        let favicon = self.project_ui.favicons.get(&project.id).cloned();
        Rc::new(move || {
            project_glyph(
                appearance.as_ref(),
                favicon.as_ref(),
                false,
                ProjectGlyphPresentation::Favicon,
            )
        })
    }
    /// Local project roots only — a favicon resolves by reading files under the
    /// project directory, which a remote workspace's root cannot offer here.
    fn project_roots(&self) -> impl Iterator<Item = (ProjectId, PathBuf)> + '_ {
        self.catalog.projects.iter().filter_map(|project| {
            let workspace = self
                .catalog
                .workspaces
                .iter()
                .find(|workspace| workspace.id == project.workspace_id)?;
            match &workspace.location {
                WorkspaceLocation::Local { root } => {
                    Some((project.id, root.join(&project.relative_directory)))
                }
                WorkspaceLocation::Ssh { .. } => None,
            }
        })
    }
    pub(super) fn refresh_project_ui(&mut self, _cx: &mut Context<Self>) {
        let workspace = self.controller.workspace.clone();
        let missing: Vec<(ProjectId, PathBuf)> = self
            .project_roots()
            .filter(|(id, _)| !self.project_ui.favicons.contains_key(id))
            .collect();
        self.job(async move {
            let uis = workspace.project_uis().await?;
            let favicons = missing
                .into_iter()
                .map(|(project, root)| {
                    let image = resolve_project_favicon(&root)
                        .and_then(|path| {
                            std::fs::read(&path).ok().and_then(|bytes| {
                                favicon_format(&path)
                                    .map(|format| gpui::Image::from_bytes(format, bytes))
                            })
                        })
                        .map(Arc::new);
                    (project, image)
                })
                .collect();
            Ok(Update::ProjectUi(Box::new(ProjectUiReply::Loaded {
                uis,
                favicons,
            })))
        });
    }
    pub(super) fn project_ui_reply(&mut self, reply: ProjectUiReply, cx: &mut Context<Self>) {
        match reply {
            ProjectUiReply::Loaded { uis, favicons } => {
                self.project_ui.uis = uis;
                let live: HashSet<ProjectId> = self
                    .catalog
                    .projects
                    .iter()
                    .map(|project| project.id)
                    .collect();
                self.project_ui
                    .favicons
                    .retain(|project, _| live.contains(project));
                for (project, image) in favicons {
                    match image {
                        Some(image) => {
                            self.project_ui.favicons.insert(project, image);
                        }
                        None => {
                            self.project_ui.favicons.remove(&project);
                        }
                    }
                }
            }
            ProjectUiReply::Saved {
                project,
                ui,
                result,
            } => match result {
                Ok(()) => {
                    if ui == ProjectUi::default() {
                        self.project_ui.uis.remove(&project);
                    } else {
                        self.project_ui.uis.insert(project, ui);
                    }
                }
                Err(error) => self.error = Some(error),
            },
        }
        cx.notify();
    }
    pub(super) fn open_project_edit(
        &mut self,
        project: ProjectId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self
            .catalog
            .projects
            .iter()
            .find(|candidate| candidate.id == project)
            .cloned()
        else {
            return;
        };
        let ui = self
            .project_ui
            .uis
            .get(&project.id)
            .cloned()
            .unwrap_or_default();
        let favicon = self.project_ui.favicons.get(&project.id).cloned();
        let dialog = cx.new(|cx| EditProjectDialog::new(project, ui, favicon, cx));
        self.project_ui.subscription = Some(cx.subscribe(&dialog, |this, _, event, cx| {
            match event {
                DialogEvent::Save {
                    project,
                    name,
                    appearance,
                } => this.save_project_ui(*project, name.clone(), appearance.clone(), cx),
                DialogEvent::Dismiss => {
                    this.project_ui.dialog = None;
                    this.project_ui.restore_focus = true;
                }
            }
            cx.notify();
        }));
        self.project_ui.previous_focus = window.focused(cx);
        self.project_ui.dialog = Some(dialog);
        cx.notify();
    }
    fn save_project_ui(
        &mut self,
        project: ProjectId,
        name: String,
        appearance: Option<ProjectAppearance>,
        cx: &mut Context<Self>,
    ) {
        self.project_ui.dialog = None;
        self.project_ui.restore_focus = true;
        let workspace = self.controller.workspace.clone();
        let ui = ProjectUi {
            name: (!name.is_empty()).then_some(name),
            appearance: normalize_project_appearance(appearance),
        };
        let ui = if ui.name.is_none() && ui.appearance.is_none() {
            ProjectUi::default()
        } else {
            ui
        };
        self.job(async move {
            let result = workspace
                .set_project_ui(project, ui.name.clone(), ui.appearance.clone())
                .await
                .map_err(|error| error.to_string());
            Ok(Update::ProjectUi(Box::new(ProjectUiReply::Saved {
                project,
                ui,
                result,
            })))
        });
        cx.notify();
    }
    pub(super) fn restore_project_ui_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_ui.restore_focus {
            self.project_ui.restore_focus = false;
            if let Some(focus) = self.project_ui.previous_focus.take() {
                window.focus(&focus, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(name: &str) -> ProjectAppearance {
        ProjectAppearance::Icon {
            icon: name.into(),
            color: None,
        }
    }

    #[test]
    fn project_glyph_source_prefers_emoji_then_custom_central_icon() {
        let emoji = ProjectAppearance::Emoji {
            emoji: "🚀".into()
        };
        assert_eq!(
            project_glyph_source(Some(&emoji), true, ProjectGlyphPresentation::Favicon,),
            ProjectGlyphSource::Emoji
        );

        let central = icon("rocket");
        assert_eq!(
            project_glyph_source(Some(&central), true, ProjectGlyphPresentation::Favicon,),
            ProjectGlyphSource::CentralIcon
        );
    }

    #[test]
    fn default_project_glyph_uses_favicon_only_for_primary_favicon_rows() {
        assert_eq!(
            project_glyph_source(None, true, ProjectGlyphPresentation::Favicon),
            ProjectGlyphSource::Favicon
        );
        assert_eq!(
            project_glyph_source(
                Some(&icon(DEFAULT_PROJECT_ICON)),
                true,
                ProjectGlyphPresentation::Favicon,
            ),
            ProjectGlyphSource::Favicon
        );
        assert_eq!(
            project_glyph_source(None, false, ProjectGlyphPresentation::Favicon),
            ProjectGlyphSource::Folder
        );
        assert_eq!(
            project_glyph_source(None, true, ProjectGlyphPresentation::Badge),
            ProjectGlyphSource::FolderBadge
        );
    }
}
