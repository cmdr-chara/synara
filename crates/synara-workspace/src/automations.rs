//! Durable automation definitions and scheduling, separate from transcript state.
//! Loading records never executes them. The scheduler must be explicitly armed
//! each process session. Claims and owned conversation creation are atomic.
mod scheduler;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService, now_ms};
use jiff::{
    Timestamp,
    tz::{Offset, TimeZone},
};
pub use scheduler::AutomationScheduler;
use serde::{Deserialize, Serialize};
use synara_core::{ProjectId, TaskId};
pub use uuid::Uuid as AutomationId;

/// Preserve the native runner's former hard-coded 15-minute deadline for
/// existing ledgers and newly created definitions.
pub const DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS: u32 = 15 * 60;
/// Keep execution bounded even though upstream contracts permit an unlimited
/// (`null`) max runtime.
pub const MAX_AUTOMATION_MAX_RUNTIME_SECONDS: u32 = 60 * 60;

fn default_automation_max_runtime_seconds() -> u32 {
    DEFAULT_AUTOMATION_MAX_RUNTIME_SECONDS
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationSchedule {
    Interval { minutes: u32 },
    Daily { hour: u8, minute: u8 },
    Weekdays { hour: u8, minute: u8 },
    Weekly { day: u8, hour: u8, minute: u8 },
    Cron { expression: String },
}
impl AutomationSchedule {
    pub fn parse(value: &str) -> WorkspaceResult<Self> {
        let value = value.trim();
        let result = if let Some(minutes) = value
            .strip_prefix("every ")
            .and_then(|s| s.strip_suffix('m'))
            .and_then(|s| s.parse().ok())
        {
            Self::Interval { minutes }
        } else if let Some(time) = value.strip_prefix("daily ") {
            let (hour, minute) = parse_time(time)?;
            Self::Daily { hour, minute }
        } else if let Some(time) = value.strip_prefix("weekdays ") {
            let (hour, minute) = parse_time(time)?;
            Self::Weekdays { hour, minute }
        } else if let Some(rest) = value.strip_prefix("weekly ") {
            let (day, time) = rest
                .split_once(' ')
                .ok_or_else(|| invalid("Use 'weekly mon 09:00'."))?;
            let day = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"]
                .iter()
                .position(|name| *name == day)
                .ok_or_else(|| invalid("Use a weekday such as mon or fri."))?
                as u8;
            let (hour, minute) = parse_time(time)?;
            Self::Weekly { day, hour, minute }
        } else if let Some(expression) = value.strip_prefix("cron ") {
            Self::Cron {
                expression: expression.trim().to_owned(),
            }
        } else {
            return Err(invalid(
                "Use 'every 60m', 'daily 09:00', 'weekdays 09:00', 'weekly mon 09:00', or 'cron 0 9 * * *'.",
            ));
        };
        result.validate()?;
        Ok(result)
    }
    pub fn label(&self) -> String {
        match self {
            Self::Interval { minutes } => format!("every {minutes}m"),
            Self::Daily { hour, minute } => format!("daily {hour:02}:{minute:02}"),
            Self::Weekdays { hour, minute } => format!("weekdays {hour:02}:{minute:02}"),
            Self::Weekly { day, hour, minute } => format!(
                "weekly {} {hour:02}:{minute:02}",
                ["sun", "mon", "tue", "wed", "thu", "fri", "sat"][usize::from(*day)]
            ),
            Self::Cron { expression } => format!("cron {expression}"),
        }
    }
    pub fn validate(&self) -> WorkspaceResult<()> {
        match self {
            Self::Interval { minutes: 1..=10080 }
            | Self::Daily {
                hour: 0..=23,
                minute: 0..=59,
            }
            | Self::Weekdays {
                hour: 0..=23,
                minute: 0..=59,
            }
            | Self::Weekly {
                day: 0..=6,
                hour: 0..=23,
                minute: 0..=59,
            }
            | Self::Cron { .. } => {
                if let Self::Cron { expression } = self {
                    parse_cron_expression(expression)?;
                }
                Ok(())
            }
            _ => Err(invalid("Schedule is outside supported limits.")),
        }
    }
    pub fn next_after(&self, now: i64, timezone: &str) -> WorkspaceResult<i64> {
        self.validate()?;
        if !(0..=253_402_214_400_000_i64).contains(&now) {
            return Err(invalid("Clock is outside supported range."));
        }
        let timezone = parse_timezone(timezone)?;
        match self {
            Self::Interval { minutes } => now
                .checked_add(i64::from(*minutes) * 60_000)
                .ok_or_else(|| invalid("Schedule overflow.")),
            Self::Daily { hour, minute }
            | Self::Weekdays { hour, minute }
            | Self::Weekly { hour, minute, .. } => {
                let hour = i8::try_from(*hour).map_err(|_| invalid("Invalid schedule hour."))?;
                let minute =
                    i8::try_from(*minute).map_err(|_| invalid("Invalid schedule minute."))?;
                let now = Timestamp::from_millisecond(now)
                    .map_err(|_| invalid("Clock is outside supported range."))?;
                let mut date = timezone.to_datetime(now).date();
                for _ in 0..=370 {
                    let weekday = date.weekday().to_sunday_zero_offset() as u8;
                    if matches!(self, Self::Weekdays { .. }) && (weekday == 0 || weekday == 6) {
                        date = date
                            .tomorrow()
                            .map_err(|_| invalid("No future schedule slot found."))?;
                        continue;
                    }
                    if let Self::Weekly { day: target, .. } = self
                        && weekday != *target
                    {
                        date = date
                            .tomorrow()
                            .map_err(|_| invalid("No future schedule slot found."))?;
                        continue;
                    }
                    let wall_time = date.at(hour, minute, 0, 0);
                    // Compatible chooses the first occurrence during a fall-back fold.
                    // In a spring-forward gap it maps to a different wall time; skipping
                    // that date avoids silently shifting the requested local schedule.
                    let candidate = timezone
                        .to_ambiguous_timestamp(wall_time)
                        .compatible()
                        .map_err(|_| invalid("No future schedule slot found."))?;
                    if timezone.to_datetime(candidate) == wall_time
                        && candidate.as_millisecond() > now.as_millisecond()
                    {
                        return Ok(candidate.as_millisecond());
                    }
                    date = date
                        .tomorrow()
                        .map_err(|_| invalid("No future schedule slot found."))?;
                }
                Err(invalid("No future schedule slot found."))
            }
            Self::Cron { expression } => {
                let cron = parse_cron_expression(expression)?;
                let now = Timestamp::from_millisecond(now)
                    .map_err(|_| invalid("Clock is outside supported range."))?;
                let mut date = timezone.to_datetime(now).date();
                for day_offset in 0..=MAX_CRON_SEARCH_DAYS {
                    if cron.date_matches(date) {
                        for hour in cron.hour.selected_values() {
                            for minute in cron.minute.selected_values() {
                                let wall_time = date.at(
                                    i8::try_from(hour)
                                        .map_err(|_| invalid("Invalid cron hour."))?,
                                    i8::try_from(minute)
                                        .map_err(|_| invalid("Invalid cron minute."))?,
                                    0,
                                    0,
                                );
                                // Compatible selects the earlier timestamp in a fold. A
                                // gap maps to a different wall time, so that occurrence is
                                // skipped. Comparing against UTC `now` also prevents the
                                // later half of a folded minute from being claimed twice.
                                let candidate = timezone
                                    .to_ambiguous_timestamp(wall_time)
                                    .compatible()
                                    .map_err(|_| invalid("No future schedule slot found."))?;
                                if timezone.to_datetime(candidate) == wall_time
                                    && candidate.as_millisecond() > now.as_millisecond()
                                {
                                    return Ok(candidate.as_millisecond());
                                }
                            }
                        }
                    }
                    if day_offset < MAX_CRON_SEARCH_DAYS {
                        date = date
                            .tomorrow()
                            .map_err(|_| invalid("No future schedule slot found."))?;
                    }
                }
                Err(invalid("No future cron slot found within eight years."))
            }
        }
    }
    pub(crate) fn advance(&self, due: i64, now: i64, timezone: &str) -> WorkspaceResult<i64> {
        match self {
            Self::Interval { minutes } => {
                self.validate()?;
                let step = i64::from(*minutes) * 60_000;
                due.checked_add(((now - due).max(0) / step + 1) * step)
                    .ok_or_else(|| invalid("Schedule overflow."))
            }
            _ => self.next_after(now, timezone),
        }
    }
}
fn parse_time(value: &str) -> WorkspaceResult<(u8, u8)> {
    let (hour, minute) = value.split_once(':').ok_or_else(|| invalid("Use HH:MM."))?;
    if hour.len() != 2
        || minute.len() != 2
        || !hour.bytes().all(|c| c.is_ascii_digit())
        || !minute.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(invalid("Use HH:MM."));
    }
    Ok((
        hour.parse().map_err(|_| invalid("Invalid hour."))?,
        minute.parse().map_err(|_| invalid("Invalid minute."))?,
    ))
}

const MAX_CRON_EXPRESSION_LENGTH: usize = 120;
const MAX_CRON_SEARCH_DAYS: u32 = 366 * 8;

#[derive(Clone, Debug)]
struct CronField {
    min: usize,
    values: Vec<bool>,
    is_wildcard: bool,
}
impl CronField {
    fn contains(&self, value: usize) -> bool {
        value
            .checked_sub(self.min)
            .and_then(|index| self.values.get(index))
            .copied()
            .unwrap_or(false)
    }
    fn selected_values(&self) -> impl Iterator<Item = usize> + '_ {
        self.values
            .iter()
            .enumerate()
            .filter_map(|(index, selected)| selected.then_some(index + self.min))
    }
}

#[derive(Clone, Debug)]
struct CronExpression {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}
impl CronExpression {
    fn date_matches(&self, date: jiff::civil::Date) -> bool {
        let day_of_month_matches = self.day_of_month.contains(date.day() as usize);
        let weekday = date.weekday().to_sunday_zero_offset() as usize;
        let day_of_week_matches = self.day_of_week.contains(weekday);
        let day_matches = if self.day_of_month.is_wildcard || self.day_of_week.is_wildcard {
            day_of_month_matches && day_of_week_matches
        } else {
            day_of_month_matches || day_of_week_matches
        };
        self.month.contains(date.month() as usize) && day_matches
    }
}

fn parse_cron_expression(expression: &str) -> WorkspaceResult<CronExpression> {
    if expression.is_empty() || expression.len() > MAX_CRON_EXPRESSION_LENGTH {
        return Err(invalid("Cron expression must be at most 120 bytes."));
    }
    let fields: Vec<_> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(invalid(
            "Cron schedules must use five fields: minute hour day-of-month month day-of-week.",
        ));
    }
    Ok(CronExpression {
        minute: parse_cron_field(fields[0], 0, 59, "minute", false)?,
        hour: parse_cron_field(fields[1], 0, 23, "hour", false)?,
        day_of_month: parse_cron_field(fields[2], 1, 31, "day-of-month", false)?,
        month: parse_cron_field(fields[3], 1, 12, "month", false)?,
        day_of_week: parse_cron_field(fields[4], 0, 7, "day-of-week", true)?,
    })
}

fn parse_cron_field(
    raw: &str,
    min: usize,
    max: usize,
    name: &str,
    day_of_week: bool,
) -> WorkspaceResult<CronField> {
    let mut values = vec![false; max - min + 1];
    let mut is_wildcard = false;
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            return Err(invalid(format!("Invalid cron {name}: empty token.")));
        }
        let mut step_parts = token.split('/');
        let range_part = step_parts.next().unwrap_or_default();
        let step = match step_parts.next() {
            Some(raw_step) => {
                let parsed = parse_cron_number(raw_step)
                    .filter(|step| *step > 0)
                    .ok_or_else(|| invalid(format!("Invalid cron {name}: bad step.")))?;
                if step_parts.next().is_some() {
                    return Err(invalid(format!("Invalid cron {name}: bad step.")));
                }
                parsed
            }
            None => 1,
        };
        if range_part == "*" && !token.contains('/') {
            is_wildcard = true;
        }
        let mut range_parts = range_part.split('-');
        let start_raw = range_parts.next().unwrap_or_default();
        let end_raw = range_parts.next().unwrap_or(start_raw);
        if range_parts.next().is_some() {
            return Err(invalid(format!("Invalid cron {name}: out of range.")));
        }
        let (start, end) = if range_part == "*" {
            (min, max)
        } else {
            (
                parse_cron_value(start_raw, day_of_week)
                    .ok_or_else(|| invalid(format!("Invalid cron {name}: out of range.")))?,
                parse_cron_value(end_raw, day_of_week)
                    .ok_or_else(|| invalid(format!("Invalid cron {name}: out of range.")))?,
            )
        };
        if start < min || start > max || end < min || end > max || start > end {
            return Err(invalid(format!("Invalid cron {name}: out of range.")));
        }
        let mut value = start;
        loop {
            let normalized = if day_of_week && value == 7 { 0 } else { value };
            values[normalized - min] = true;
            let Some(next) = value.checked_add(step) else {
                break;
            };
            if next > end {
                break;
            }
            value = next;
        }
    }
    Ok(CronField {
        min,
        values,
        is_wildcard,
    })
}

fn parse_cron_value(raw: &str, day_of_week: bool) -> Option<usize> {
    if day_of_week {
        match raw.to_ascii_lowercase().as_str() {
            "sun" => return Some(0),
            "mon" => return Some(1),
            "tue" => return Some(2),
            "wed" => return Some(3),
            "thu" => return Some(4),
            "fri" => return Some(5),
            "sat" => return Some(6),
            _ => {}
        }
    }
    parse_cron_number(raw)
}

fn parse_cron_number(raw: &str) -> Option<usize> {
    (!raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}
/// Parse a fixed offset. Named zones are handled by `parse_timezone` so DST
/// rules are applied to each local calendar date instead of approximated.
pub fn timezone_offset(value: &str) -> WorkspaceResult<i32> {
    if value == "UTC" {
        return Ok(0);
    }
    if value.len() != 6
        || !matches!(value.as_bytes()[0], b'+' | b'-')
        || value.as_bytes()[3] != b':'
    {
        return Err(invalid(
            "Timezone is not a fixed offset. Use UTC, an offset such as +02:00, or an IANA zone such as Europe/Rome.",
        ));
    }
    let hours: i32 = value
        .get(1..3)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid("Invalid timezone offset."))?;
    let minutes: i32 = value
        .get(4..6)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid("Invalid timezone offset."))?;
    if hours > 14 || minutes > 59 || hours < 0 || minutes < 0 || (hours == 14 && minutes != 0) {
        return Err(invalid("Invalid timezone offset."));
    }
    Ok((hours * 60 + minutes) * if value.starts_with('-') { -1 } else { 1 })
}

fn parse_timezone(value: &str) -> WorkspaceResult<TimeZone> {
    if value == "UTC" {
        return Ok(TimeZone::UTC);
    }
    if let Ok(minutes) = timezone_offset(value) {
        let offset =
            Offset::from_seconds(minutes * 60).map_err(|_| invalid("Invalid timezone offset."))?;
        return Ok(TimeZone::fixed(offset));
    }

    // Only accept a bounded IANA identifier, never a path or POSIX TZ rule.
    // Segment checks also reject traversal and empty path components before
    // handing the identifier to the timezone database.
    let safe_identifier = !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
        });
    if !safe_identifier {
        return Err(invalid(
            "Use UTC, a fixed offset, or a valid IANA timezone name.",
        ));
    }
    TimeZone::get(value).map_err(|_| invalid("Unknown IANA timezone name."))
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    CatchUpOnce,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationDefinition {
    pub id: AutomationId,
    pub revision: u64,
    pub title: String,
    pub instructions: String,
    pub agent_id: String,
    pub project_id: ProjectId,
    pub schedule: AutomationSchedule,
    pub timezone: String,
    pub enabled: bool,
    pub next_run_ms: i64,
    pub missed: MissedRunPolicy,
    #[serde(default)]
    pub max_runs: Option<u32>,
    #[serde(default)]
    pub stop_after_consecutive_failures: Option<u32>,
    #[serde(default)]
    pub failure_streak: u32,
    #[serde(default = "default_automation_max_runtime_seconds")]
    pub max_runtime_seconds: u32,
}
impl AutomationDefinition {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.title.trim().is_empty()
            || self.title.len() > 200
            || self.title.chars().any(char::is_control)
            || self.instructions.trim().is_empty()
            || self.instructions.len() > 16 * 1024
            || self.instructions.contains('\0')
            || self.agent_id.is_empty()
            || self.agent_id.len() > 256
            || self.agent_id.chars().any(char::is_control)
            || !(0..=253_402_300_799_000_i64).contains(&self.next_run_ms)
            || self.max_runs == Some(0)
            || self.stop_after_consecutive_failures == Some(0)
            || !(1..=MAX_AUTOMATION_MAX_RUNTIME_SECONDS).contains(&self.max_runtime_seconds)
        {
            return Err(invalid(
                "Automation needs a title, instructions (up to 16 KiB), explicit agent and project.",
            ));
        }
        self.schedule.validate()?;
        parse_timezone(&self.timezone)?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRunStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
    Skipped,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationRun {
    pub id: AutomationId,
    pub definition: AutomationDefinition,
    pub owner: AutomationId,
    pub scheduled_ms: Option<i64>,
    pub started_ms: i64,
    pub finished_ms: Option<i64>,
    pub status: AutomationRunStatus,
    pub task_id: Option<TaskId>,
    pub output: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationLedger {
    pub version: u32,
    pub definitions: Vec<AutomationDefinition>,
    pub runs: Vec<AutomationRun>,
}
impl Default for AutomationLedger {
    fn default() -> Self {
        Self {
            version: 1,
            definitions: vec![],
            runs: vec![],
        }
    }
}
impl AutomationLedger {
    pub(crate) fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 || self.definitions.len() > 64 || self.runs.len() > 256 {
            return Err(invalid(
                "Unsupported automation ledger or capacity reached (64 definitions / 256 retained runs).",
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for d in &self.definitions {
            d.validate()?;
            if !ids.insert(d.id) {
                return Err(invalid("Duplicate automation identity."));
            }
        }
        ids.clear();
        for r in &self.runs {
            r.definition.validate()?;
            if !ids.insert(r.id) || r.output.len() > 16 * 1024 {
                return Err(invalid("Invalid run history."));
            }
        }
        Ok(())
    }
}
pub(crate) fn invalid(message: impl Into<String>) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
impl WorkspaceService {
    pub async fn automations(&self) -> WorkspaceResult<AutomationLedger> {
        self.access(|store| store.automation_ledger()).await
    }
    /// Saving always pauses. Re-enabling is a separate explicit action.
    pub async fn save_automation(
        &self,
        mut definition: AutomationDefinition,
        expected_revision: Option<u64>,
    ) -> WorkspaceResult<()> {
        definition.validate()?;
        self.access(move |store| {
            store.validate_automation_context(&definition)?;
            store.edit_automations(move |ledger| {
                definition.enabled = false;
                definition.next_run_ms = definition
                    .schedule
                    .next_after(now_ms(), &definition.timezone)?;
                match (
                    ledger
                        .definitions
                        .iter()
                        .position(|d| d.id == definition.id),
                    expected_revision,
                ) {
                    (None, None) => {
                        definition.revision = 1;
                        ledger.definitions.push(definition);
                    }
                    (Some(index), Some(revision))
                        if ledger.definitions[index].revision == revision =>
                    {
                        if ledger.runs.iter().any(|r| {
                            r.definition.id == definition.id
                                && r.status == AutomationRunStatus::Running
                        }) {
                            return Err(invalid("Stop the active run before editing."));
                        }
                        definition.revision = revision
                            .checked_add(1)
                            .ok_or_else(|| invalid("Revision overflow."))?;
                        ledger.definitions[index] = definition;
                    }
                    _ => {
                        return Err(invalid(
                            "Automation changed in another window. Reload before saving.",
                        ));
                    }
                }
                Ok(())
            })
        })
        .await
    }
    pub async fn enable_automation(
        &self,
        id: AutomationId,
        revision: u64,
        enabled: bool,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let completed_runs = ledger
                    .runs
                    .iter()
                    .filter(|run| run.definition.id == id && run.task_id.is_some())
                    .count();
                let definition = ledger
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == id && d.revision == revision)
                    .ok_or_else(|| invalid("Automation changed. Reload first."))?;
                if enabled
                    && definition
                        .max_runs
                        .is_some_and(|max| completed_runs >= max as usize)
                {
                    return Err(invalid(
                        "Run limit reached. Increase the limit before resuming.",
                    ));
                }
                definition.enabled = enabled;
                if enabled {
                    definition.failure_streak = 0;
                    definition.next_run_ms = definition
                        .schedule
                        .next_after(now_ms(), &definition.timezone)?;
                }
                definition.revision = definition
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| invalid("Revision overflow."))?;
                Ok(())
            })
        })
        .await
    }
    pub async fn delete_automation(
        &self,
        id: AutomationId,
        revision: u64,
        confirmed: bool,
    ) -> WorkspaceResult<()> {
        if !confirmed {
            return Err(invalid("Deletion requires explicit confirmation."));
        }
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                if ledger
                    .runs
                    .iter()
                    .any(|r| r.definition.id == id && r.status == AutomationRunStatus::Running)
                {
                    return Err(invalid("Stop or resolve the active run before deleting."));
                }
                let index = ledger
                    .definitions
                    .iter()
                    .position(|d| d.id == id && d.revision == revision)
                    .ok_or_else(|| invalid("Automation changed. Reload first."))?;
                ledger.definitions.remove(index);
                // Historical snapshots and generated tasks remain owned and inspectable.
                Ok(())
            })
        })
        .await
    }
    #[cfg(test)]
    pub(crate) async fn claim_automation(
        &self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        self.claim_automation_revision(id, owner, scheduled, now, None)
            .await
    }
    pub(crate) async fn claim_automation_revision(
        &self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
        revision: Option<u64>,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        self.access(move |store| store.claim_automation(id, owner, scheduled, now, revision))
            .await
    }
    pub(crate) async fn finish_automation(
        &self,
        id: AutomationId,
        owner: AutomationId,
        status: AutomationRunStatus,
        output: String,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let run = ledger
                    .runs
                    .iter_mut()
                    .find(|r| {
                        r.id == id && r.owner == owner && r.status == AutomationRunStatus::Running
                    })
                    .ok_or_else(|| invalid("Run ownership changed."))?;
                if matches!(
                    status,
                    AutomationRunStatus::Running | AutomationRunStatus::Skipped
                ) {
                    return Err(invalid("Invalid run completion."));
                }
                run.status = status;
                run.finished_ms = Some(now_ms());
                run.output = output.chars().take(4096).collect();
                let definition_id = run.definition.id;
                if let Some(definition) = ledger
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == definition_id)
                {
                    if status == AutomationRunStatus::Failed {
                        definition.failure_streak = definition.failure_streak.saturating_add(1);
                        if definition
                            .stop_after_consecutive_failures
                            .is_some_and(|limit| definition.failure_streak >= limit)
                        {
                            definition.enabled = false;
                            definition.revision = definition
                                .revision
                                .checked_add(1)
                                .ok_or_else(|| invalid("Revision overflow."))?;
                        }
                    } else {
                        definition.failure_streak = 0;
                    }
                }
                Ok(())
            })
        })
        .await
    }
    /// Explicit recovery after the user has verified the prior app/process stopped.
    /// Never resubmits a claimed slot. Also pauses its definition.
    pub async fn resolve_interrupted_automation(
        &self,
        id: AutomationId,
        current_owner: AutomationId,
        confirmed_stopped: bool,
    ) -> WorkspaceResult<()> {
        if !confirmed_stopped {
            return Err(invalid(
                "Verify that the previous process has stopped, then confirm.",
            ));
        }
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let run = ledger
                    .runs
                    .iter_mut()
                    .find(|r| {
                        r.id == id
                            && r.owner != current_owner
                            && r.status == AutomationRunStatus::Running
                    })
                    .ok_or_else(|| {
                        invalid("Only a previous process's unresolved run can be recovered.")
                    })?;
                run.status = AutomationRunStatus::Interrupted;
                run.finished_ms = Some(now_ms());
                run.output =
                    "Previous process stopped. External outcome is unknown. No automatic retry."
                        .into();
                if let Some(definition) = ledger
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == run.definition.id)
                {
                    definition.enabled = false;
                    definition.revision = definition
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| invalid("Revision overflow."))?;
                }
                Ok(())
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests;
