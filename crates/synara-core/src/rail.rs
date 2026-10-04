//! Pure rules for the rail layout's tab strip: item ids, item order and
//! visibility, pinned Space/project shortcuts, which item is active for the
//! current surface, and the Spaces panel's section list. Mirrors upstream
//! `appRail.logic.ts` plus the reconcile half of `railShellStore.ts`.
use crate::ProjectId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// App-level sidebar layout: one column ("classic") or the fixed icon rail
/// plus a collapsible panel column ("rail"). Upstream resolves to classic on
/// mobile; the native shell only has the desktop form factor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarLayout {
    #[default]
    Classic,
    Rail,
}

/// Rail items that switch the panel content instead of navigating.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailPanelItemId {
    Home,
    Spaces,
}
impl RailPanelItemId {
    pub const ALL: [Self; 2] = [Self::Home, Self::Spaces];
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Spaces => "Spaces",
        }
    }
}

/// Rail items that navigate to a destination. "New thread" stays in the
/// panel, never the rail.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailRouteItemId {
    Kanban,
    PullRequests,
    Automations,
    Studio,
    Settings,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RailItemId {
    Panel(RailPanelItemId),
    Route(RailRouteItemId),
}
impl RailItemId {
    pub const HOME: Self = Self::Panel(RailPanelItemId::Home);
    pub const SPACES: Self = Self::Panel(RailPanelItemId::Spaces);
    pub const KANBAN: Self = Self::Route(RailRouteItemId::Kanban);
    pub const PULL_REQUESTS: Self = Self::Route(RailRouteItemId::PullRequests);
    pub const AUTOMATIONS: Self = Self::Route(RailRouteItemId::Automations);
    pub const STUDIO: Self = Self::Route(RailRouteItemId::Studio);
    pub const SETTINGS: Self = Self::Route(RailRouteItemId::Settings);
}

/// Rail items the user orders and hides from Customize. Settings stays pinned
/// at the bottom; Studio is also reachable from the "…" menu, so it starts
/// hidden.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailOrderableItemId {
    Home,
    Spaces,
    Kanban,
    PullRequests,
    Automations,
    Studio,
}
impl RailOrderableItemId {
    pub const ALL: [Self; 6] = [
        Self::Home,
        Self::Spaces,
        Self::Kanban,
        Self::PullRequests,
        Self::Automations,
        Self::Studio,
    ];
    /// Studio needs its section enabled in Settings before it can show.
    pub const DEFAULT_HIDDEN: [Self; 1] = [Self::Studio];
    pub fn item(self) -> RailItemId {
        match self {
            Self::Home => RailItemId::HOME,
            Self::Spaces => RailItemId::SPACES,
            Self::Kanban => RailItemId::KANBAN,
            Self::PullRequests => RailItemId::PULL_REQUESTS,
            Self::Automations => RailItemId::AUTOMATIONS,
            Self::Studio => RailItemId::STUDIO,
        }
    }
    pub fn from_item(item: RailItemId) -> Option<Self> {
        match item {
            RailItemId::Panel(RailPanelItemId::Home) => Some(Self::Home),
            RailItemId::Panel(RailPanelItemId::Spaces) => Some(Self::Spaces),
            RailItemId::Route(RailRouteItemId::Kanban) => Some(Self::Kanban),
            RailItemId::Route(RailRouteItemId::PullRequests) => Some(Self::PullRequests),
            RailItemId::Route(RailRouteItemId::Automations) => Some(Self::Automations),
            RailItemId::Route(RailRouteItemId::Studio) => Some(Self::Studio),
            RailItemId::Route(RailRouteItemId::Settings) => None,
        }
    }
    /// The persisted id string (upstream's camelCase `RailOrderableItemId`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Spaces => "spaces",
            Self::Kanban => "kanban",
            Self::PullRequests => "pullRequests",
            Self::Automations => "automations",
            Self::Studio => "studio",
        }
    }
}

/// Home is the rail's anchor for the panel: it can move but never be hidden.
pub fn rail_item_can_hide(id: RailOrderableItemId) -> bool {
    id != RailOrderableItemId::Home
}

fn known_orderable_ids(ids: &[String]) -> Vec<RailOrderableItemId> {
    let mut seen = HashSet::new();
    ids.iter()
        .filter_map(|id| parse_orderable(id))
        .filter(|id| seen.insert(*id))
        .collect()
}
fn parse_orderable(id: &str) -> Option<RailOrderableItemId> {
    RailOrderableItemId::ALL
        .into_iter()
        .find(|candidate| candidate.name() == id)
}

/// A complete order: the saved known ids first, then any default id the saved
/// order lacks (items shipped after the user persisted an order), appended in
/// default order.
pub fn normalize_rail_item_order(order: &[String]) -> Vec<RailOrderableItemId> {
    let mut result = known_orderable_ids(order);
    for id in RailOrderableItemId::ALL {
        if !result.contains(&id) {
            result.push(id);
        }
    }
    result
}

/// Persisted hidden ids, de-duplicated, minus Home which can never hide.
pub fn normalize_hidden_rail_items(hidden: &[String]) -> Vec<RailOrderableItemId> {
    known_orderable_ids(hidden)
        .into_iter()
        .filter(|id| rail_item_can_hide(*id))
        .collect()
}

/// The rail's top items in the user's order. Hidden items drop out unless
/// they are the active item, so hiding a section never strands the user in
/// it; Studio needs its section enabled in Settings.
pub fn build_rail_item_order(
    order: &[RailOrderableItemId],
    hidden: &HashSet<RailOrderableItemId>,
    active_item: RailItemId,
    studio_available: bool,
) -> Vec<RailOrderableItemId> {
    let active_orderable = RailOrderableItemId::from_item(active_item);
    order
        .iter()
        .copied()
        .filter(|id| {
            (*id != RailOrderableItemId::Studio || studio_available)
                && (!hidden.contains(id) || Some(*id) == active_orderable)
        })
        .collect()
}

/// Key the Void section uses in persisted shortcuts and group keys. Same value
/// as upstream `RESERVED_VOID_SPACE_ID`.
pub const VOID_SPACE_KEY: &str = "void";
const SPACE_SHORTCUT_PREFIX: &str = "space:";
const PROJECT_SHORTCUT_PREFIX: &str = "project:";

/// A Space or a single project the user added to the rail from its "…" menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RailShortcut {
    Space {
        key: String,
        space_id: Option<String>,
    },
    Project {
        key: String,
        project_id: ProjectId,
    },
}
impl RailShortcut {
    pub fn key(&self) -> &str {
        match self {
            Self::Space { key, .. } | Self::Project { key, .. } => key,
        }
    }
}

/// Narrows an optional Space id to the string key that stands in for it.
pub fn space_key(space_id: Option<&str>) -> String {
    space_id.unwrap_or(VOID_SPACE_KEY).to_owned()
}
pub fn rail_space_shortcut_key(space_id: Option<&str>) -> String {
    format!("{SPACE_SHORTCUT_PREFIX}{}", space_key(space_id))
}
pub fn rail_project_shortcut_key(project_id: ProjectId) -> String {
    format!("{PROJECT_SHORTCUT_PREFIX}{project_id}")
}

/// The persisted shortcut keys that still point at something, in their saved
/// order: unknown, deleted, and duplicate entries drop out (Void always
/// exists).
pub fn resolve_rail_shortcuts(
    keys: &[String],
    space_ids: &HashSet<String>,
    project_ids: &HashSet<ProjectId>,
) -> Vec<RailShortcut> {
    let mut seen = HashSet::new();
    let mut shortcuts = Vec::new();
    for key in keys {
        if !seen.insert(key.as_str()) {
            continue;
        }
        if let Some(id) = key.strip_prefix(SPACE_SHORTCUT_PREFIX) {
            if id == VOID_SPACE_KEY {
                shortcuts.push(RailShortcut::Space {
                    key: key.clone(),
                    space_id: None,
                });
            } else if space_ids.contains(id) {
                shortcuts.push(RailShortcut::Space {
                    key: key.clone(),
                    space_id: Some(id.to_owned()),
                });
            }
        } else if let Some(id) = key.strip_prefix(PROJECT_SHORTCUT_PREFIX)
            && let Ok(uuid) = uuid::Uuid::parse_str(id)
        {
            let project_id = ProjectId(uuid);
            if project_ids.contains(&project_id) {
                shortcuts.push(RailShortcut::Project {
                    key: key.clone(),
                    project_id,
                });
            }
        }
    }
    shortcuts
}

/// Adds a shortcut at the end of the rail, or removes it when it is already
/// there.
pub fn toggle_rail_shortcut_key(keys: &[String], key: &str) -> Vec<String> {
    if keys.iter().any(|entry| entry == key) {
        keys.iter()
            .filter(|entry| entry.as_str() != key)
            .cloned()
            .collect()
    } else {
        keys.iter()
            .cloned()
            .chain(std::iter::once(key.to_owned()))
            .collect()
    }
}

/// The shortcut that stands for what the panel shows, so exactly one rail
/// item is active: a pinned Space while Home shows that Space, a pinned
/// project while Spaces shows its threads.
pub fn resolve_active_rail_shortcut_key(
    active_item: RailItemId,
    active_space_id: Option<&str>,
    spaces_project_id: Option<ProjectId>,
    shortcuts: &[RailShortcut],
) -> Option<String> {
    for shortcut in shortcuts {
        match shortcut {
            RailShortcut::Space { key, space_id }
                if active_item == RailItemId::HOME && space_id.as_deref() == active_space_id =>
            {
                return Some(key.clone());
            }
            RailShortcut::Project { key, project_id }
                if active_item == RailItemId::SPACES && Some(*project_id) == spaces_project_id =>
            {
                return Some(key.clone());
            }
            _ => {}
        }
    }
    None
}

/// Whether the panel column shows next to the rail for the active item. Every
/// section either owns a panel (Home/Spaces: projects and threads;
/// Automations, Studio, Settings: their own lists) or takes the full width:
/// Kanban is one board, Pull requests has its own list and detail panes.
pub fn rail_item_shows_panel(id: RailItemId) -> bool {
    id != RailItemId::KANBAN && id != RailItemId::PULL_REQUESTS
}

/// The route rail item that owns a surface, or None for thread and chat-index
/// surfaces (upstream `railItemForPathname` on the surface's route).
pub fn rail_item_for_destination(destination: RailRouteItemId) -> RailItemId {
    RailItemId::Route(destination)
}

/// Re-syncs the active item after navigation: a route item wins when the
/// surface is its destination (shortcut, deep link, command palette); a
/// Studio surface counts as the Studio route; anywhere else the current
/// panel item is active, so exactly one rail item is active at a time.
/// (Upstream `reconcileActiveRailItem`; `pathname` resolves to its route item
/// upstream, so the caller passes the destination's item directly.)
pub fn reconcile_active_rail_item(
    current: RailItemId,
    destination: Option<RailRouteItemId>,
    on_studio_surface: bool,
    panel_view: RailPanelItemId,
) -> RailItemId {
    let _ = current;
    destination.map_or_else(
        || {
            if on_studio_surface {
                RailItemId::STUDIO
            } else {
                RailItemId::Panel(panel_view)
            }
        },
        RailItemId::Route,
    )
}

/// One section of the Spaces panel: the space it groups and its projects.
#[derive(Clone, Debug)]
pub struct RailSpacesSection<T> {
    pub key: String,
    pub space_id: Option<String>,
    pub name: String,
    pub items: Vec<T>,
}

/// Space ids in the order every grouped project list presents them: the
/// active space first, then Void, then the remaining spaces in their
/// user-defined order.
fn ordered_space_ids_for_picker(
    space_ids: &[String],
    active_space_id: Option<&str>,
) -> Vec<Option<String>> {
    let mut ordered = Vec::with_capacity(space_ids.len() + 1);
    ordered.push(active_space_id.map(str::to_owned));
    ordered.push(None);
    ordered.extend(space_ids.iter().cloned().map(Some));
    let mut seen = HashSet::new();
    ordered.retain(|id| seen.insert(space_key(id.as_deref())));
    ordered
}

/// Spaces panel sections in the shared picker order (active space, Void, the
/// rest, then spaces the snapshot has not caught up with). Empty spaces stay
/// listed so they can show their empty state; an empty Void is dropped
/// because it is only the unfiled bucket, unless it is the only section there
/// is. (Upstream `buildRailSpacesSections` over `groupItemsBySpace`.)
pub fn build_rail_spaces_sections<T>(
    items: Vec<T>,
    space_ids: &[String],
    space_name: impl Fn(Option<&str>) -> String,
    active_space_id: Option<&str>,
    space_id_of: impl Fn(&T) -> Option<String>,
    void_name: &str,
) -> Vec<RailSpacesSection<T>> {
    let mut groups: BTreeMap<String, (Option<String>, String, Vec<T>)> = BTreeMap::new();
    let mut group_order: Vec<String> = Vec::new();
    for item in items {
        let space_id = space_id_of(&item);
        let key = space_key(space_id.as_deref());
        if !groups.contains_key(&key) {
            groups.insert(
                key.clone(),
                (
                    space_id.clone(),
                    space_name(space_id.as_deref()),
                    Vec::new(),
                ),
            );
            group_order.push(key.clone());
        }
        groups.get_mut(&key).unwrap().2.push(item);
    }
    let mut ordered_keys = HashSet::new();
    let mut sections = Vec::new();
    for space_id in ordered_space_ids_for_picker(space_ids, active_space_id) {
        let key = space_key(space_id.as_deref());
        ordered_keys.insert(key.clone());
        match groups.get_mut(&key) {
            Some((_, name, items)) => {
                let items = std::mem::take(items);
                sections.push(RailSpacesSection {
                    key,
                    space_id,
                    name: name.clone(),
                    items,
                });
            }
            None => {
                // An empty Void is only the unfiled bucket and drops; a named
                // Space stays listed so it can show its empty state.
                if space_id.is_none() {
                    continue;
                }
                sections.push(RailSpacesSection {
                    key,
                    name: space_name(space_id.as_deref()),
                    space_id,
                    items: Vec::new(),
                });
            }
        }
    }
    for key in group_order {
        if ordered_keys.contains(&key) {
            continue;
        }
        let (space_id, name, items) = groups.remove(&key).unwrap();
        sections.push(RailSpacesSection {
            key,
            space_id,
            name,
            items,
        });
    }
    if sections.is_empty() {
        sections.push(RailSpacesSection {
            key: space_key(None),
            space_id: None,
            name: void_name.to_owned(),
            items: Vec::new(),
        });
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(ids: &[RailOrderableItemId]) -> Vec<&'static str> {
        ids.iter().map(|id| id.name()).collect()
    }

    #[test]
    fn rail_item_order_normalizes_known_ids_then_appends_missing_defaults() {
        let order = normalize_rail_item_order(&[
            "spaces".to_owned(),
            "bogus".to_owned(),
            "spaces".to_owned(),
            "kanban".to_owned(),
        ]);
        assert_eq!(
            names(&order),
            vec![
                "spaces",
                "kanban",
                "home",
                "pullRequests",
                "automations",
                "studio"
            ]
        );
    }

    #[test]
    fn hidden_items_never_include_home() {
        let hidden = normalize_hidden_rail_items(&[
            "home".to_owned(),
            "studio".to_owned(),
            "bogus".to_owned(),
            "studio".to_owned(),
        ]);
        assert_eq!(hidden, vec![RailOrderableItemId::Studio]);
    }

    #[test]
    fn rail_order_drops_hidden_unless_active_and_studio_unless_available() {
        let order = normalize_rail_item_order(&[]);
        let hidden: HashSet<_> = [RailOrderableItemId::Studio, RailOrderableItemId::Spaces]
            .into_iter()
            .collect();
        let visible = build_rail_item_order(&order, &hidden, RailItemId::HOME, true);
        assert_eq!(
            names(&visible),
            vec!["home", "kanban", "pullRequests", "automations"]
        );
        // The active item survives being hidden.
        let visible = build_rail_item_order(&order, &hidden, RailItemId::SPACES, true);
        assert_eq!(
            names(&visible),
            vec!["home", "spaces", "kanban", "pullRequests", "automations"]
        );
        // Studio requires its enabled section even while unhidden.
        let visible = build_rail_item_order(&order, &HashSet::new(), RailItemId::STUDIO, false);
        assert!(!visible.contains(&RailOrderableItemId::Studio));
        let visible = build_rail_item_order(&order, &hidden, RailItemId::STUDIO, true);
        assert!(visible.contains(&RailOrderableItemId::Studio));
    }

    #[test]
    fn shortcuts_resolve_known_targets_in_saved_order() {
        let project = ProjectId::new();
        let keys = vec![
            rail_space_shortcut_key(Some("alpha")),
            rail_project_shortcut_key(project),
            "space:ghost".to_owned(),
            "project:not-a-uuid".to_owned(),
            rail_space_shortcut_key(None),
            rail_space_shortcut_key(Some("alpha")),
            "junk".to_owned(),
        ];
        let space_ids: HashSet<String> = ["alpha".to_owned()].into_iter().collect();
        let project_ids: HashSet<ProjectId> = [project].into_iter().collect();
        let shortcuts = resolve_rail_shortcuts(&keys, &space_ids, &project_ids);
        assert_eq!(
            shortcuts,
            vec![
                RailShortcut::Space {
                    key: "space:alpha".to_owned(),
                    space_id: Some("alpha".to_owned()),
                },
                RailShortcut::Project {
                    key: format!("project:{project}"),
                    project_id: project,
                },
                RailShortcut::Space {
                    key: "space:void".to_owned(),
                    space_id: None,
                },
            ]
        );
    }

    #[test]
    fn shortcut_toggle_adds_or_removes_once() {
        let keys = vec!["space:void".to_owned()];
        let toggled = toggle_rail_shortcut_key(&keys, "project:abc");
        assert_eq!(
            toggled,
            vec!["space:void".to_owned(), "project:abc".to_owned()]
        );
        let toggled = toggle_rail_shortcut_key(&toggled, "space:void");
        assert_eq!(toggled, vec!["project:abc".to_owned()]);
    }

    #[test]
    fn active_shortcut_stands_for_panel_content_only() {
        let project = ProjectId::new();
        let shortcuts = vec![
            RailShortcut::Space {
                key: "space:alpha".to_owned(),
                space_id: Some("alpha".to_owned()),
            },
            RailShortcut::Project {
                key: format!("project:{project}"),
                project_id: project,
            },
        ];
        assert_eq!(
            resolve_active_rail_shortcut_key(RailItemId::HOME, Some("alpha"), None, &shortcuts),
            Some("space:alpha".to_owned())
        );
        // Home is not the Spaces panel: the pinned project does not claim it.
        assert_eq!(
            resolve_active_rail_shortcut_key(
                RailItemId::HOME,
                Some("alpha"),
                Some(project),
                &shortcuts
            ),
            Some("space:alpha".to_owned())
        );
        assert_eq!(
            resolve_active_rail_shortcut_key(
                RailItemId::SPACES,
                Some("alpha"),
                Some(project),
                &shortcuts
            ),
            Some(format!("project:{project}"))
        );
        assert_eq!(
            resolve_active_rail_shortcut_key(
                RailItemId::AUTOMATIONS,
                Some("alpha"),
                Some(project),
                &shortcuts
            ),
            None
        );
    }

    #[test]
    fn panel_visibility_follows_destination_ownership() {
        assert!(rail_item_shows_panel(RailItemId::HOME));
        assert!(rail_item_shows_panel(RailItemId::AUTOMATIONS));
        assert!(rail_item_shows_panel(RailItemId::SETTINGS));
        assert!(!rail_item_shows_panel(RailItemId::KANBAN));
        assert!(!rail_item_shows_panel(RailItemId::PULL_REQUESTS));
    }

    #[test]
    fn reconcile_prefers_destination_then_studio_then_panel() {
        assert_eq!(
            reconcile_active_rail_item(
                RailItemId::HOME,
                Some(RailRouteItemId::Automations),
                false,
                RailPanelItemId::Home,
            ),
            RailItemId::AUTOMATIONS
        );
        assert_eq!(
            reconcile_active_rail_item(RailItemId::HOME, None, true, RailPanelItemId::Home),
            RailItemId::STUDIO
        );
        assert_eq!(
            reconcile_active_rail_item(
                RailItemId::AUTOMATIONS,
                None,
                false,
                RailPanelItemId::Spaces,
            ),
            RailItemId::SPACES
        );
    }

    #[test]
    fn spaces_sections_order_active_void_rest_then_orphans() {
        let space_name = |id: Option<&str>| match id {
            None => "Void".to_owned(),
            Some("alpha") => "Alpha".to_owned(),
            Some("beta") => "Beta".to_owned(),
            Some(_) => "Unknown space".to_owned(),
        };
        let sections = build_rail_spaces_sections(
            vec!["a1", "b1", "v1", "ghost1"],
            &["alpha".to_owned(), "beta".to_owned()],
            space_name,
            Some("alpha"),
            |item| match *item {
                "a1" => Some("alpha".to_owned()),
                "b1" => Some("beta".to_owned()),
                "ghost1" => Some("gone".to_owned()),
                _ => None,
            },
            "Void",
        );
        let keys: Vec<_> = sections
            .iter()
            .map(|section| section.key.as_str())
            .collect();
        assert_eq!(keys, vec!["alpha", "void", "beta", "gone"]);
        let counts: Vec<_> = sections.iter().map(|section| section.items.len()).collect();
        assert_eq!(counts, vec![1, 1, 1, 1]);
        // Void drops when empty; an entirely empty list still gets one Void.
        let sections = build_rail_spaces_sections(
            Vec::<&str>::new(),
            &["alpha".to_owned()],
            space_name,
            Some("alpha"),
            |_| None,
            "Void",
        );
        let keys: Vec<_> = sections
            .iter()
            .map(|section| section.key.as_str())
            .collect();
        assert_eq!(keys, vec!["alpha"]);
        let sections =
            build_rail_spaces_sections(Vec::<&str>::new(), &[], space_name, None, |_| None, "Void");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].key, "void");
    }
}
