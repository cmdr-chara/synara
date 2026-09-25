//! Local profile activity visualizations built from durable thread turn timestamps.
use super::*;
use chrono::Datelike;

// Match the upstream profile's bounded 274-day activity window.
const HEATMAP_WINDOW_DAYS: i64 = 274;
const MILLIS_PER_DAY: i64 = 86_400_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct HeatmapCell {
    epoch_day: i64,
    count: u64,
    intensity: u8,
    date_label: String,
    month_label: String,
    month_key: Option<(i32, u32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HeatmapColumn {
    month_label: String,
    cells: Vec<Option<HeatmapCell>>,
}

/// Build a Sunday-first calendar for a fixed, bounded UTC window.
/// Padding cells are `None`, so dates before the window and after `today` never
/// acquire misleading zero counts.
fn heatmap_columns(days: &BTreeMap<i64, u64>, today: i64) -> Vec<HeatmapColumn> {
    let first_day = today.saturating_sub(HEATMAP_WINDOW_DAYS - 1);
    let leading_padding = weekday_sunday_first(first_day);
    let mut cells = Vec::with_capacity(leading_padding + HEATMAP_WINDOW_DAYS as usize + 6);
    cells.extend((0..leading_padding).map(|_| None));

    let mut active_counts = (first_day..=today)
        .filter_map(|day| days.get(&day).copied())
        .filter(|count| *count > 0)
        .collect::<Vec<_>>();
    active_counts.sort_unstable();

    for day in first_day..=today {
        let count = days.get(&day).copied().unwrap_or_default();
        let (date_label, month_label, month_key) = date_labels(day);
        cells.push(Some(HeatmapCell {
            epoch_day: day,
            count,
            intensity: heatmap_intensity(count, &active_counts),
            date_label,
            month_label,
            month_key,
        }));
    }
    while cells.len() % 7 != 0 {
        cells.push(None);
    }

    let mut previous_month = None;
    cells
        .as_chunks::<7>()
        .0
        .iter()
        .map(|week| {
            let first_date = week.iter().flatten().find_map(|cell| {
                cell.month_key
                    .map(|month| (month, cell.month_label.as_str()))
            });
            let month_label = match first_date {
                Some((month, month_name)) if previous_month != Some(month) => {
                    previous_month = Some(month);
                    month_name.to_owned()
                }
                _ => String::new(),
            };
            HeatmapColumn {
                month_label,
                cells: week.to_vec(),
            }
        })
        .collect()
}

fn weekday_sunday_first(epoch_day: i64) -> usize {
    // 1970-01-01 was Thursday (index 4). Taking the remainder first avoids
    // overflowing when this helper is exercised with extreme i64 values.
    ((epoch_day.rem_euclid(7) + 4) % 7) as usize
}

fn date_labels(epoch_day: i64) -> (String, String, Option<(i32, u32)>) {
    let millis = epoch_day.saturating_mul(MILLIS_PER_DAY);
    let Some(date) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)
        .map(|timestamp| timestamp.date_naive())
    else {
        return (format!("UTC day {epoch_day}"), String::new(), None);
    };
    (
        date.format("%A, %B %-d, %Y").to_string(),
        date.format("%b").to_string(),
        Some((date.year(), date.month())),
    )
}

fn heatmap_intensity(count: u64, sorted_active_counts: &[u64]) -> u8 {
    if count == 0 || sorted_active_counts.is_empty() {
        return 0;
    }
    let rank = sorted_active_counts.partition_point(|active_count| *active_count <= count);
    // Rank among non-empty days, like the upstream profile heatmap. Ties stay
    // together and a uniform window uses the top level.
    rank.saturating_mul(4)
        .div_ceil(sorted_active_counts.len())
        .clamp(1, 4) as u8
}

fn heatmap_color(level: u8) -> u32 {
    if level == 0 {
        return palette().border;
    }
    let from = palette().canvas;
    let to = palette().focus;
    let percent = [24_u32, 46, 72, 100][usize::from(level.min(4) - 1)];
    let channel = |shift: u32| {
        let from = (from >> shift) & 0xff_u32;
        let to = (to >> shift) & 0xff_u32;
        (from * (100 - percent) + to * percent) / 100
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// Resolve the last model selection recorded in a thread's durable session
/// configuration. This is a thread-level snapshot, not a claim about each turn.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct SessionModel {
    pub value: String,
    pub label: String,
}

pub(super) fn current_session_model(configuration: &SessionConfiguration) -> Option<SessionModel> {
    let mut model_options = configuration
        .options
        .iter()
        .filter(|option| option.category.as_deref() == Some("model"));
    if let Some(option) = model_options.next() {
        if model_options.next().is_some() {
            // There is no truthful single-model summary for multiple independent selectors.
            return None;
        }
        let ConfigValue::Select { value } = &option.current else {
            return None;
        };
        let label = option
            .choices
            .iter()
            .find(|choice| choice.value == *value)
            .map(|choice| {
                if choice.label.trim().is_empty() {
                    value.clone()
                } else {
                    choice.label.clone()
                }
            })
            .unwrap_or_else(|| value.clone());
        return Some(SessionModel {
            value: value.clone(),
            label,
        });
    }

    let value = configuration.current_model.as_ref()?;
    if value.trim().is_empty() {
        return None;
    }
    let label = configuration
        .models
        .iter()
        .find(|model| model.value == *value)
        .map(|model| {
            if model.label.trim().is_empty() {
                value.clone()
            } else {
                model.label.clone()
            }
        })
        .unwrap_or_else(|| value.clone());
    Some(SessionModel {
        value: value.clone(),
        label,
    })
}

pub(super) fn record_model_selection(
    activity: &mut ProfileActivity,
    agent_id: &str,
    configuration: &SessionConfiguration,
) {
    let model = current_session_model(configuration);
    let count = activity
        .model_selections
        .entry((agent_id.to_owned(), model))
        .or_default();
    *count = count.saturating_add(1);
}

pub(super) fn saved_model_selections(
    activity: Option<&ProfileActivity>,
    profiles: &[AgentProfile],
    loading: bool,
) -> gpui::AnyElement {
    let Some(activity) = activity else {
        return div()
            .text_color(rgb(palette().muted))
            .child(if loading {
                "Loading local model selections…"
            } else {
                "Local model selections have not been loaded yet."
            })
            .into_any_element();
    };

    let mut rows: Vec<_> = activity
        .model_selections
        .iter()
        .map(|((agent, model), threads)| (agent.as_str(), model.as_ref(), *threads))
        .collect();
    rows.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| left.0.cmp(right.0))
            .then_with(|| {
                left.1
                    .map(|model| model.label.as_str())
                    .cmp(&right.1.map(|model| model.label.as_str()))
            })
            .then_with(|| {
                left.1
                    .map(|model| model.value.as_str())
                    .cmp(&right.1.map(|model| model.value.as_str()))
            })
    });
    let total_threads = rows
        .iter()
        .fold(0_usize, |total, row| total.saturating_add(row.2));
    let recorded_threads = rows
        .iter()
        .filter(|row| row.1.is_some())
        .fold(0_usize, |total, row| total.saturating_add(row.2));
    let missing_threads = total_threads.saturating_sub(recorded_threads);
    if total_threads == 0 {
        return div()
            .text_color(rgb(palette().muted))
            .child("No local thread model selections have been recorded.")
            .into_any_element();
    }

    let displayed_rows = rows.len().min(8);
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(format!(
                    "Latest saved session-config model per local thread · {total_threads} threads · {recorded_threads} with a model recorded · {missing_threads} without one recorded"
                )),
        )
        .children(rows.iter().take(displayed_rows).map(|(agent, model, count)| {
            let agent_label = profiles
                .iter()
                .find(|profile| profile.id == *agent)
                .map(|profile| profile.name.clone())
                .unwrap_or_else(|| format!("Unknown agent ({agent})"));
            let model_label = model.map_or_else(
                || "Model not recorded".to_owned(),
                |model| {
                    if model.value == model.label {
                        model.label.clone()
                    } else {
                        format!("{} · {}", model.label, model.value)
                    }
                },
            );
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_ellipsis()
                        .text_size(px(12.))
                        .child(format!("{agent_label} · {model_label}")),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(11.))
                        .text_color(rgb(palette().muted))
                        .child(format!("{count} threads")),
                )
        }))
        .children((rows.len() > displayed_rows).then(|| {
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(format!("{} more saved selections", rows.len() - displayed_rows))
        }))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child("One count per local thread, using its latest persisted session configuration. Direct-model route selections are stored separately and are not included. This is not per-turn model history, provider usage, account telemetry, or billing."),
        )
        .into_any_element()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeatmapMetric {
    Tokens,
    Prompts,
}

fn activity_heatmap_series(activity: &ProfileActivity) -> (&BTreeMap<i64, u64>, HeatmapMetric) {
    if activity.token_days.values().any(|count| *count > 0) {
        (&activity.token_days, HeatmapMetric::Tokens)
    } else {
        (&activity.days, HeatmapMetric::Prompts)
    }
}

pub(super) fn activity_heatmap(activity: &ProfileActivity, today: i64) -> gpui::AnyElement {
    let (days, metric) = activity_heatmap_series(activity);
    let columns = heatmap_columns(days, today);
    let (group_label, group_description, summary, disclosure) = match metric {
        HeatmapMetric::Tokens => (
            "Local token activity heatmap",
            "Each labeled square is a UTC date and the number of provider-reported tokens from persisted turns. Weeks begin Sunday.",
            "Provider-reported token activity · last 274 days · UTC dates and week boundaries",
            "Token cells include only turns with both provider-reported input and output token counts; turns without token telemetry are omitted.",
        ),
        HeatmapMetric::Prompts => (
            "Local turn activity heatmap",
            "Each labeled square is a UTC date and the number of persisted turn starts. Weeks begin Sunday.",
            "Local workspace turn starts · last 274 days · UTC dates and week boundaries",
            "Counts come from persisted turn timestamps on this installation; they are not provider or account usage.",
        ),
    };
    let month_row = div()
        .w_full()
        .flex()
        .gap(px(2.))
        .children(columns.iter().map(|column| {
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(9.))
                .text_color(rgb(palette().muted))
                .child(column.month_label.clone())
        }));
    let grid = div()
        .id("profile-turn-heatmap-grid")
        .role(gpui::Role::List)
        .aria_label("Dates in the local activity heatmap")
        .w_full()
        .flex()
        .gap(px(2.))
        .children(columns.iter().enumerate().map(|(week_index, column)| {
            div()
                .id(("profile-turn-heatmap-week", week_index))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(column.cells.iter().enumerate().map(|(slot_index, cell)| {
                    let slot_id = week_index * 7 + slot_index;
                    let Some(cell) = cell else {
                        return div()
                            .id(("profile-turn-heatmap-padding", slot_id))
                            .w_full()
                            .h(px(10.));
                    };
                    let noun = match (metric, cell.count) {
                        (HeatmapMetric::Tokens, 1) => "provider-reported token",
                        (HeatmapMetric::Tokens, _) => "provider-reported tokens",
                        (HeatmapMetric::Prompts, 1) => "turn start",
                        (HeatmapMetric::Prompts, _) => "turn starts",
                    };
                    let label = format!("{}: {} {noun}", cell.date_label, cell.count);
                    div()
                        .id(("profile-turn-heatmap-day", cell.epoch_day as u64))
                        .w_full()
                        .h(px(10.))
                        .rounded(px(3.))
                        .bg(rgb(heatmap_color(cell.intensity)))
                        .role(gpui::Role::ListItem)
                        .aria_label(label.clone())
                        .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(label.clone().into())).into())
                }))
        }));

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .id("profile-turn-heatmap")
                .role(gpui::Role::Group)
                .aria_label(group_label)
                .aria_description(group_description)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(month_row)
                .child(grid),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(summary),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(disclosure),
        )
        .into_any_element()
}

pub(super) fn active_hours(activity: Option<&ProfileActivity>, loading: bool) -> gpui::AnyElement {
    let Some(activity) = activity else {
        return div()
            .text_color(rgb(palette().muted))
            .child(if loading {
                "Loading local activity…"
            } else {
                "Activity has not been loaded yet."
            })
            .into_any_element();
    };

    let max_count = activity.hours.iter().copied().max().unwrap_or_default();
    let peak = activity
        .hours
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .filter(|(_, count)| **count > 0)
        .map(|(hour, count)| format!("{hour:02}:00 UTC · {count} prompts"))
        .unwrap_or_else(|| "No recorded turn starts.".into());

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette().muted))
                .child(peak),
        )
        .child(
            div()
                .id("profile-active-hours")
                .w_full()
                .h(px(82.))
                .flex()
                .items_end()
                .gap(px(3.))
                .children(activity.hours.iter().enumerate().map(|(hour, count)| {
                    let height = if max_count == 0 || *count == 0 {
                        2.
                    } else {
                        6. + (*count as f32 / max_count as f32) * 68.
                    };
                    div()
                        .id(("profile-active-hour", hour))
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_end()
                        .child(div().w_full().h(px(height)).rounded(px(3.)).bg(rgb(
                            if *count > 0 {
                                palette().focus
                            } else {
                                palette().border
                            },
                        )))
                })),
        )
        .child(div().w_full().flex().children((0..24).map(|hour| {
            div()
                .flex_1()
                .text_size(px(10.))
                .text_color(rgb(palette().muted))
                .child(if hour % 6 == 0 {
                    format!("{hour:02}")
                } else {
                    String::new()
                })
        })))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child("Turn start times from this installation · UTC"),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(columns: &[HeatmapColumn]) -> Vec<&HeatmapCell> {
        columns
            .iter()
            .flat_map(|column| column.cells.iter().flatten())
            .collect()
    }

    #[test]
    fn calendar_is_sunday_first_and_bounded_to_upstream_window() {
        let columns = heatmap_columns(&BTreeMap::new(), 363);

        assert_eq!(weekday_sunday_first(0), 4); // 1970-01-01, Thursday.
        assert_eq!(weekday_sunday_first(3), 0); // 1970-01-04, Sunday.
        assert_eq!(weekday_sunday_first(-1), 3); // 1969-12-31, Wednesday.
        assert_eq!(columns.len(), 40); // 274 dates plus calendar alignment.
        assert!(columns[0].cells[..3].iter().all(Option::is_none));
        assert_eq!(columns[0].cells[3].as_ref().unwrap().epoch_day, 90);

        let visible = cells(&columns);
        assert_eq!(visible.len(), HEATMAP_WINDOW_DAYS as usize);
        assert_eq!(visible.first().unwrap().epoch_day, 90);
        assert_eq!(visible.last().unwrap().epoch_day, 363);
        assert!(columns.iter().all(|column| column.cells.len() == 7));
    }

    #[test]
    fn calendar_keeps_zero_dates_but_ignores_history_outside_the_window() {
        let today = 1_000;
        let first = today - HEATMAP_WINDOW_DAYS + 1;
        let days = BTreeMap::from([
            (first - 1, 9),
            (first, 2),
            (first + 1, 0),
            (today, 4),
            (today + 1, 8),
        ]);

        let columns = heatmap_columns(&days, today);
        let visible = cells(&columns);

        assert_eq!(visible.len(), HEATMAP_WINDOW_DAYS as usize);
        assert_eq!(visible[0].count, 2);
        assert_eq!(visible[1].count, 0);
        assert_eq!(visible.last().unwrap().count, 4);
        assert_eq!(visible[0].intensity, 2);
        assert_eq!(visible[1].intensity, 0);
        assert_eq!(visible.last().unwrap().intensity, 4);
        assert!(
            visible
                .iter()
                .all(|cell| (first..=today).contains(&cell.epoch_day))
        );
    }

    #[test]
    fn intensity_ranks_active_days_and_preserves_ties() {
        let sorted = [1, 2, 3, 4, 100];

        assert_eq!(heatmap_intensity(0, &sorted), 0);
        assert_eq!(heatmap_intensity(1, &sorted), 1);
        assert_eq!(heatmap_intensity(2, &sorted), 2);
        assert_eq!(heatmap_intensity(3, &sorted), 3);
        assert_eq!(heatmap_intensity(100, &sorted), 4);
        assert_eq!(heatmap_intensity(500, &[500, 500, 500]), 4);
    }

    #[test]
    fn date_labels_name_the_utc_day_and_month() {
        let (date, month, key) = date_labels(0);
        assert_eq!(date, "Thursday, January 1, 1970");
        assert_eq!(month, "Jan");
        assert_eq!(key, Some((1970, 1)));

        assert_eq!(date_labels(-1).0, "Wednesday, December 31, 1969");
        assert_eq!(date_labels(18_321).0, "Saturday, February 29, 2020");
    }

    #[test]
    fn month_labels_repeat_when_the_calendar_year_changes() {
        let columns = heatmap_columns(&BTreeMap::new(), 385);
        let month_labels = columns
            .iter()
            .filter(|column| !column.month_label.is_empty())
            .map(|column| column.month_label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(month_labels.first(), Some(&"Apr"));
        assert_eq!(month_labels.last(), Some(&"Jan"));
        assert!(
            month_labels
                .windows(2)
                .any(|months| months == ["Dec", "Jan"])
        );
    }

    #[test]
    fn activity_heatmap_prefers_reported_tokens_and_falls_back_to_turns() {
        let mut activity = ProfileActivity::default();
        activity.days.insert(10, 3);

        let (days, metric) = activity_heatmap_series(&activity);
        assert_eq!(metric, HeatmapMetric::Prompts);
        assert_eq!(days.get(&10), Some(&3));

        activity.token_days.insert(10, 4_500);
        let (days, metric) = activity_heatmap_series(&activity);
        assert_eq!(metric, HeatmapMetric::Tokens);
        assert_eq!(days.get(&10), Some(&4_500));
    }

    #[test]
    fn model_snapshot_uses_the_persisted_model_option_before_legacy_fields() {
        let configuration = SessionConfiguration {
            current_model: Some("stale-legacy-model".into()),
            models: vec![SelectChoice {
                value: "stale-legacy-model".into(),
                label: "Stale legacy model".into(),
                group: None,
            }],
            options: vec![SessionOption {
                id: "model-choice".into(),
                name: "Model".into(),
                description: None,
                category: Some("model".into()),
                current: ConfigValue::Select {
                    value: "current-model".into(),
                },
                choices: vec![SelectChoice {
                    value: "current-model".into(),
                    label: "Current model".into(),
                    group: None,
                }],
            }],
            ..Default::default()
        };

        assert_eq!(
            current_session_model(&configuration),
            Some(SessionModel {
                value: "current-model".into(),
                label: "Current model".into(),
            })
        );
    }

    #[test]
    fn model_snapshot_uses_legacy_advertised_model_label_when_available() {
        let configuration = SessionConfiguration {
            current_model: Some("model-id".into()),
            models: vec![SelectChoice {
                value: "model-id".into(),
                label: "Model display name".into(),
                group: None,
            }],
            ..Default::default()
        };
        assert_eq!(
            current_session_model(&configuration),
            Some(SessionModel {
                value: "model-id".into(),
                label: "Model display name".into(),
            })
        );
    }

    #[test]
    fn model_snapshot_keeps_missing_and_ambiguous_values_unreported() {
        let stale_fallback = SessionConfiguration {
            current_model: Some("legacy-model".into()),
            options: vec![
                SessionOption {
                    id: "first-model".into(),
                    name: "First model".into(),
                    description: None,
                    category: Some("model".into()),
                    current: ConfigValue::Select {
                        value: "one".into(),
                    },
                    choices: vec![],
                },
                SessionOption {
                    id: "second-model".into(),
                    name: "Second model".into(),
                    description: None,
                    category: Some("model".into()),
                    current: ConfigValue::Select {
                        value: "two".into(),
                    },
                    choices: vec![],
                },
            ],
            ..Default::default()
        };
        assert_eq!(current_session_model(&stale_fallback), None);
        assert_eq!(
            current_session_model(&SessionConfiguration::default()),
            None
        );
    }

    #[test]
    fn saved_model_counts_keep_missing_selections_separate() {
        let config = SessionConfiguration {
            current_model: Some("model-id".into()),
            models: vec![SelectChoice {
                value: "model-id".into(),
                label: "Model display name".into(),
                group: None,
            }],
            ..Default::default()
        };
        let mut activity = ProfileActivity::default();
        record_model_selection(&mut activity, "codex", &config);
        record_model_selection(&mut activity, "codex", &config);
        record_model_selection(
            &mut activity,
            "custom-agent",
            &SessionConfiguration::default(),
        );

        assert_eq!(
            activity.model_selections.get(&(
                "codex".into(),
                Some(SessionModel {
                    value: "model-id".into(),
                    label: "Model display name".into(),
                })
            )),
            Some(&2)
        );
        assert_eq!(
            activity
                .model_selections
                .get(&("custom-agent".into(), None)),
            Some(&1)
        );
    }
}
