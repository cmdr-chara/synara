//! Upstream `projectAppearance.ts`: the local look of a project — a curated
//! Central icon in a palette color, or an emoji. Stored per project as a
//! renderer preference; `None` is the default folder and persists nothing.
use serde::{Deserialize, Serialize};

/// Upstream `PROJECT_COLORS` (order preserved — the picker shows this row).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
}
pub const PROJECT_COLORS: [ProjectColor; 7] = [
    ProjectColor::Red,
    ProjectColor::Orange,
    ProjectColor::Yellow,
    ProjectColor::Green,
    ProjectColor::Blue,
    ProjectColor::Purple,
    ProjectColor::Pink,
];
impl ProjectColor {
    /// The persisted value (upstream `ProjectColor` union member).
    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
        }
    }
    /// Upstream `PROJECT_COLOR_LABELS`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Pink => "Pink",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        PROJECT_COLORS
            .iter()
            .copied()
            .find(|color| color.name() == name)
    }
    /// Upstream `--project-*`: the Tailwind 500 swatch in light mode, a 72%/28%
    /// white mix in dark (yellow dark uses `yellow-400` like upstream).
    pub fn rgb(self, dark_theme: bool) -> u32 {
        let (light, dark) = match self {
            Self::Red => (0xef4444, 0xf37878),
            Self::Orange => (0xf97316, 0xfb9a57),
            Self::Yellow => (0xeab308, 0xfbda57),
            Self::Green => (0x22c55e, 0x60d58b),
            Self::Blue => (0x3b82f6, 0x72a5f9),
            Self::Purple => (0x8b5cf6, 0xac8af9),
            Self::Pink => (0xec4899, 0xf17bb6),
        };
        if dark_theme { dark } else { light }
    }
}

pub struct ProjectIconOption {
    /// Central icon asset name; also the persisted value.
    pub name: &'static str,
    pub label: &'static str,
    /// Extra search words beyond the label.
    pub keywords: &'static str,
}

/// Upstream `DEFAULT_PROJECT_ICON`; the folder every project shows by default.
pub const DEFAULT_PROJECT_ICON: &str = "folder-2";

/// Upstream `PROJECT_ICON_OPTIONS` in picker order.
pub const PROJECT_ICON_OPTIONS: &[ProjectIconOption] = &[
    ProjectIconOption {
        name: "folder-2",
        label: "Folder",
        keywords: "default directory project",
    },
    ProjectIconOption {
        name: "dollar",
        label: "Money",
        keywords: "finance dollar budget cash",
    },
    ProjectIconOption {
        name: "book",
        label: "Book",
        keywords: "reading docs library",
    },
    ProjectIconOption {
        name: "graduate-cap",
        label: "Education",
        keywords: "school study course learn",
    },
    ProjectIconOption {
        name: "pencil",
        label: "Writing",
        keywords: "edit draft blog",
    },
    ProjectIconOption {
        name: "feather",
        label: "Pen",
        keywords: "writing poetry quill",
    },
    ProjectIconOption {
        name: "brackets-2",
        label: "Code",
        keywords: "braces json dev programming",
    },
    ProjectIconOption {
        name: "console",
        label: "Terminal",
        keywords: "shell cli command",
    },
    ProjectIconOption {
        name: "audio",
        label: "Music",
        keywords: "song sound note",
    },
    ProjectIconOption {
        name: "popcorn",
        label: "Movies",
        keywords: "film cinema entertainment",
    },
    ProjectIconOption {
        name: "ruler",
        label: "Design",
        keywords: "layout measure architecture",
    },
    ProjectIconOption {
        name: "color-palette",
        label: "Art",
        keywords: "paint colors creative",
    },
    ProjectIconOption {
        name: "heart-beat",
        label: "Health",
        keywords: "medical doctor pulse",
    },
    ProjectIconOption {
        name: "medicine-pill",
        label: "Medicine",
        keywords: "pharmacy pills care",
    },
    ProjectIconOption {
        name: "form-flower",
        label: "Wellness",
        keywords: "calm lotus mindfulness",
    },
    ProjectIconOption {
        name: "suitcase-work",
        label: "Work",
        keywords: "briefcase job business office",
    },
    ProjectIconOption {
        name: "chart-3",
        label: "Analytics",
        keywords: "chart stats data metrics",
    },
    ProjectIconOption {
        name: "dumbell",
        label: "Fitness",
        keywords: "gym workout sport",
    },
    ProjectIconOption {
        name: "notebook",
        label: "Notes",
        keywords: "journal notebook diary",
    },
    ProjectIconOption {
        name: "law",
        label: "Law",
        keywords: "legal scale justice balance",
    },
    ProjectIconOption {
        name: "globe",
        label: "Globe",
        keywords: "web internet world",
    },
    ProjectIconOption {
        name: "airplane",
        label: "Travel",
        keywords: "plane trip flight",
    },
    ProjectIconOption {
        name: "earth",
        label: "Earth",
        keywords: "planet world climate",
    },
    ProjectIconOption {
        name: "maintenance",
        label: "Tools",
        keywords: "wrench fix settings repair",
    },
    ProjectIconOption {
        name: "pets",
        label: "Pets",
        keywords: "paw dog cat animal",
    },
    ProjectIconOption {
        name: "lab",
        label: "Science",
        keywords: "flask lab research experiment",
    },
    ProjectIconOption {
        name: "brain",
        label: "Brain",
        keywords: "mind ai thinking research",
    },
    ProjectIconOption {
        name: "heart",
        label: "Heart",
        keywords: "love favorite personal",
    },
    ProjectIconOption {
        name: "tree",
        label: "Nature",
        keywords: "plant garden tree green",
    },
    ProjectIconOption {
        name: "rocket",
        label: "Launch",
        keywords: "rocket startup ship",
    },
    ProjectIconOption {
        name: "light-bulb",
        label: "Idea",
        keywords: "lightbulb ideas brainstorm",
    },
    ProjectIconOption {
        name: "star",
        label: "Star",
        keywords: "favorite important",
    },
    ProjectIconOption {
        name: "camera-1",
        label: "Photo",
        keywords: "camera pictures photography",
    },
    ProjectIconOption {
        name: "gamecontroller",
        label: "Games",
        keywords: "gaming play controller",
    },
    ProjectIconOption {
        name: "home",
        label: "Home",
        keywords: "house personal family",
    },
    ProjectIconOption {
        name: "people",
        label: "People",
        keywords: "team user community",
    },
    ProjectIconOption {
        name: "robot",
        label: "Bot",
        keywords: "robot ai agent automation",
    },
    ProjectIconOption {
        name: "cup-hot",
        label: "Coffee",
        keywords: "tea break drink",
    },
    ProjectIconOption {
        name: "shopping-bag-1",
        label: "Shopping",
        keywords: "store shop ecommerce",
    },
    ProjectIconOption {
        name: "bug",
        label: "Bugs",
        keywords: "debug issue qa",
    },
    ProjectIconOption {
        name: "server",
        label: "Server",
        keywords: "backend infra database hosting",
    },
    ProjectIconOption {
        name: "puzzle",
        label: "Puzzle",
        keywords: "plugin extension piece",
    },
    ProjectIconOption {
        name: "trophy",
        label: "Trophy",
        keywords: "win award goal",
    },
    ProjectIconOption {
        name: "target",
        label: "Goal",
        keywords: "target focus objective",
    },
    ProjectIconOption {
        name: "map-pin",
        label: "Place",
        keywords: "location map local",
    },
    ProjectIconOption {
        name: "chat-bubbles",
        label: "Chat",
        keywords: "messages conversation support",
    },
    ProjectIconOption {
        name: "calendar-1",
        label: "Calendar",
        keywords: "schedule date plan events",
    },
    ProjectIconOption {
        name: "lightning",
        label: "Speed",
        keywords: "fast bolt power energy",
    },
];

/// Upstream `ProjectAppearance` — serialized as the same JSON the web renderer
/// persists (`{"kind":"icon","icon":"rocket","color":"blue"}` or
/// `{"kind":"emoji","emoji":"🚀"}`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ProjectAppearance {
    #[serde(rename = "icon")]
    Icon {
        icon: String,
        color: Option<ProjectColorName>,
    },
    #[serde(rename = "emoji")]
    Emoji { emoji: String },
}

/// Serialized color: upstream stores the union's string value, not an object.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectColorName(pub String);
impl ProjectColorName {
    pub fn color(&self) -> Option<ProjectColor> {
        ProjectColor::from_name(&self.0)
    }
}
impl From<ProjectColor> for ProjectColorName {
    fn from(color: ProjectColor) -> Self {
        Self(color.name().to_owned())
    }
}

/// Upstream `normalizeProjectAppearance`: the default folder without a color
/// means "no appearance" so a reset leaves nothing behind in storage.
pub fn normalize_project_appearance(
    appearance: Option<ProjectAppearance>,
) -> Option<ProjectAppearance> {
    match appearance {
        Some(ProjectAppearance::Icon { icon, color })
            if icon == DEFAULT_PROJECT_ICON && color.is_none() =>
        {
            None
        }
        appearance => appearance,
    }
}

/// Upstream `projectAppearanceKey` — stable string identity for caches.
pub fn project_appearance_key(appearance: Option<&ProjectAppearance>) -> String {
    match appearance {
        None => "default".to_owned(),
        Some(ProjectAppearance::Icon { icon, color }) => format!(
            "icon:{icon}:{}",
            color.as_ref().map_or("", |color| color.0.as_str())
        ),
        Some(ProjectAppearance::Emoji { emoji }) => format!("emoji:{emoji}"),
    }
}

pub fn project_appearance_equals(
    left: Option<&ProjectAppearance>,
    right: Option<&ProjectAppearance>,
) -> bool {
    project_appearance_key(left) == project_appearance_key(right)
}

fn is_regional_indicator(c: char) -> bool {
    ('\u{1f1e6}'..='\u{1f1ff}').contains(&c)
}

/// Covers the blocks upstream's `\p{Extended_Pictographic}` test exercises for
/// any emoji a user realistically picks: the emoji planes, Dingbats, Misc
/// Symbols, Transport/Map symbols, Supplemental Arrows-B dingbats, select BMP
/// sign posts, and ZWJ/skin-tone helpers piggyback on a pictographic base.
fn is_extended_pictographic(c: char) -> bool {
    matches!(c,
        '\u{00a9}' | '\u{00ae}' | '\u{203c}' | '\u{2049}' | '\u{2122}' | '\u{2139}'
        | '\u{2194}'..='\u{2199}' | '\u{21a9}'..='\u{21aa}'
        | '\u{231a}'..='\u{231b}' | '\u{2328}' | '\u{23cf}'
        | '\u{23e9}'..='\u{23f3}' | '\u{23f8}'..='\u{23fa}'
        | '\u{24c2}' | '\u{25aa}'..='\u{25ab}' | '\u{25b6}' | '\u{25c0}'
        | '\u{25fb}'..='\u{25fe}' | '\u{2600}'..='\u{27bf}'
        | '\u{2934}'..='\u{2935}' | '\u{2b05}'..='\u{2b07}' | '\u{2b1b}'..='\u{2b1c}'
        | '\u{2b50}' | '\u{2b55}' | '\u{3030}' | '\u{303d}' | '\u{3297}' | '\u{3299}'
        | '\u{1f000}'..='\u{1fbff}' | '\u{1fc00}'..='\u{1fffd}')
}

/// Upstream `firstEmoji`: the first grapheme carrying a pictographic code
/// point, a regional indicator, or the keycap U+20E3 — one grapheme keeps skin
/// tones, flags and ZWJ sequences whole.
pub fn first_emoji(value: &str) -> Option<String> {
    use unicode_segmentation::UnicodeSegmentation;
    for segment in value.graphemes(true) {
        if segment
            .chars()
            .any(|c| is_extended_pictographic(c) || is_regional_indicator(c) || c == '\u{20e3}')
        {
            return Some(segment.to_owned());
        }
    }
    None
}

/// Upstream `parseProjectAppearance`: validates a persisted appearance;
/// anything unknown or malformed falls back to the default folder.
pub fn parse_project_appearance(
    appearance: Option<ProjectAppearance>,
) -> Option<ProjectAppearance> {
    match appearance {
        Some(ProjectAppearance::Icon { icon, color }) => {
            if !PROJECT_ICON_OPTIONS
                .iter()
                .any(|option| option.name == icon)
            {
                return None;
            }
            let color = color
                .and_then(|color| color.color())
                .map(ProjectColorName::from);
            normalize_project_appearance(Some(ProjectAppearance::Icon { icon, color }))
        }
        Some(ProjectAppearance::Emoji { emoji }) => match first_emoji(&emoji) {
            Some(first) if first == emoji => Some(ProjectAppearance::Emoji { emoji: first }),
            _ => None,
        },
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(icon: &str, color: Option<ProjectColor>) -> Option<ProjectAppearance> {
        Some(ProjectAppearance::Icon {
            icon: icon.to_owned(),
            color: color.map(Into::into),
        })
    }

    #[test]
    fn normalize_drops_default_folder_without_color() {
        assert_eq!(normalize_project_appearance(icon("folder-2", None)), None);
        assert!(normalize_project_appearance(icon("folder-2", Some(ProjectColor::Red))).is_some());
        assert!(normalize_project_appearance(icon("rocket", None)).is_some());
    }

    #[test]
    fn appearance_key_matches_upstream_shape() {
        assert_eq!(project_appearance_key(None), "default");
        assert_eq!(
            project_appearance_key(icon("rocket", Some(ProjectColor::Blue)).as_ref()),
            "icon:rocket:blue"
        );
        assert_eq!(
            project_appearance_key(Some(&ProjectAppearance::Emoji {
                emoji: "🚀".into()
            })),
            "emoji:🚀"
        );
    }

    #[test]
    fn parse_rejects_unknown_icon_and_non_emoji() {
        assert_eq!(parse_project_appearance(icon("not-an-icon", None)), None);
        assert_eq!(
            parse_project_appearance(Some(ProjectAppearance::Emoji {
                emoji: "hello".into()
            })),
            None
        );
        // An emoji plus trailing text is not a single emoji.
        assert_eq!(
            parse_project_appearance(Some(ProjectAppearance::Emoji {
                emoji: "🚀 ship".into()
            })),
            None
        );
        // Unknown colors decode but validate to None -> icon without color.
        let decoded: Option<ProjectAppearance> =
            serde_json::from_str(r#"{"kind":"icon","icon":"rocket","color":"plaid"}"#).ok();
        assert_eq!(parse_project_appearance(decoded), icon("rocket", None));
    }

    #[test]
    fn parse_accepts_icon_color_and_single_emoji() {
        assert_eq!(
            parse_project_appearance(icon("rocket", Some(ProjectColor::Pink))),
            icon("rocket", Some(ProjectColor::Pink))
        );
        assert_eq!(
            parse_project_appearance(Some(ProjectAppearance::Emoji {
                emoji: "1\u{fe0f}\u{20e3}".into()
            })),
            Some(ProjectAppearance::Emoji {
                emoji: "1\u{fe0f}\u{20e3}".into()
            })
        );
        // Flag (regional-indicator pair) parses as one grapheme.
        assert_eq!(
            parse_project_appearance(Some(ProjectAppearance::Emoji {
                emoji: "🇫🇷".into()
            })),
            Some(ProjectAppearance::Emoji {
                emoji: "🇫🇷".into()
            })
        );
    }

    #[test]
    fn persisted_shape_matches_upstream_json() {
        let icon = serde_json::to_value(icon("rocket", Some(ProjectColor::Blue))).unwrap();
        assert_eq!(
            icon,
            serde_json::json!({"kind":"icon","icon":"rocket","color":"blue"})
        );
        let emoji = serde_json::to_value(ProjectAppearance::Emoji {
            emoji: "🌳".into()
        })
        .unwrap();
        assert_eq!(emoji, serde_json::json!({"kind":"emoji","emoji":"🌳"}));
    }
}
