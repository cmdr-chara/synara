//! Proposed plans: upstream `apps/web/src/proposedPlan.ts` plus the
//! `extractProposedPlanMarkdown` helper from `apps/server/src/provider/planMode.ts`.
//! A completed plan-mode turn wraps its final plan in `<proposed_plan>` tags; the
//! extracted markdown becomes a `thread.proposedPlans` record that drives the
//! "Plan ready" composer banner and the same-thread implementation submit.
use serde::{Deserialize, Serialize};

/// Upstream `OrchestrationProposedPlan` (packages/contracts orchestration.ts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedPlan {
    /// `plan:{thread_id}:turn:{turn_id}` — same shape as `proposedPlanIdFromEvent`.
    pub id: String,
    pub turn_id: Option<String>,
    pub plan_markdown: String,
    pub implemented_at_ms: Option<i64>,
    pub implementation_thread_id: Option<crate::ThreadId>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// Upstream `SourceProposedPlanReference`: attached to the turn that
/// implements a plan, pointing back at the source thread's plan record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProposedPlan {
    pub thread_id: crate::ThreadId,
    pub plan_id: String,
}

/// Upstream `PROPOSED_PLAN_BLOCK_REGEX` match: first
/// `<proposed_plan>\s*(…)\s*</proposed_plan>` capture, trimmed, non-empty.
pub fn extract_proposed_plan_markdown(text: &str) -> Option<String> {
    let (content_start, content_end) = first_proposed_plan_block(text)?;
    let markdown = text[content_start..content_end].trim();
    (!markdown.is_empty()).then(|| markdown.to_owned())
}

/// Upstream `stripProposedPlanBlocksFromText`: remove every
/// `<proposed_plan>\s*…\s*</proposed_plan>` block (case-insensitive), then trim.
pub fn strip_proposed_plan_blocks_from_text(text: &str) -> String {
    let mut out = text.to_owned();
    while let Some((content_start, content_end)) = first_proposed_plan_block(&out) {
        let block_start = content_start - PROPOSED_PLAN_OPEN.len();
        let block_end = content_end + PROPOSED_PLAN_CLOSE.len();
        out.replace_range(block_start..block_end, "");
    }
    out.trim().to_owned()
}

/// Upstream `proposedPlanTitle`: first `#{1,6}` heading line (≤3 leading spaces).
pub fn proposed_plan_title(plan_markdown: &str) -> Option<String> {
    for line in plan_markdown.lines() {
        let Some(body) = heading_body(line) else {
            continue;
        };
        return (!body.is_empty()).then(|| body.to_owned());
    }
    None
}

/// Upstream `stripDisplayedPlanMarkdown`: drop the leading title heading (and a
/// following `# Summary` heading) before rendering the card body.
pub fn strip_displayed_plan_markdown(plan_markdown: &str) -> String {
    let lines = plan_markdown.trim_end().split('\n');
    let mut lines: Vec<&str> = lines
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    if lines.first().is_some_and(|line| is_heading_line(line)) {
        lines.remove(0);
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines
        .first()
        .and_then(|line| heading_body(line))
        .is_some_and(|body| body.eq_ignore_ascii_case("summary"))
    {
        lines.remove(0);
        while lines.first().is_some_and(|line| line.trim().is_empty()) {
            lines.remove(0);
        }
    }
    lines.join("\n")
}

/// Upstream `buildCollapsedProposedPlanPreviewMarkdown` (default `maxLines: 8`).
pub fn build_collapsed_proposed_plan_preview_markdown(plan_markdown: &str) -> String {
    collapsed_proposed_plan_preview(plan_markdown, 8)
}
/// Upstream `buildCollapsedProposedPlanPreviewMarkdown` with an explicit
/// `maxLines` (the transcript card passes 10).
pub fn collapsed_proposed_plan_preview(plan_markdown: &str, max_lines: usize) -> String {
    let lines = strip_displayed_plan_markdown(plan_markdown);
    let mut preview: Vec<String> = Vec::new();
    let mut visible = 0usize;
    let mut has_more = false;
    for line in lines.trim_end().split('\n') {
        let line = line.trim_end();
        let is_visible = !line.trim().is_empty();
        if is_visible && visible >= max_lines {
            has_more = true;
            break;
        }
        preview.push(line.to_owned());
        visible += usize::from(is_visible);
    }
    while preview.last().is_some_and(|line| line.trim().is_empty()) {
        preview.pop();
    }
    if preview.is_empty() {
        return proposed_plan_title(plan_markdown)
            .unwrap_or_else(|| "Plan preview unavailable.".into());
    }
    if has_more {
        preview.push(String::new());
        preview.push("...".into());
    }
    preview.join("\n")
}

/// Upstream `buildPlanImplementationPrompt`.
pub fn build_plan_implementation_prompt(plan_markdown: &str) -> String {
    format!("PLEASE IMPLEMENT THIS PLAN:\n{}", plan_markdown.trim())
}

/// Upstream `resolvePlanFollowUpSubmission`: a non-empty draft stays a plan-mode
/// refinement; an empty draft submits the implementation prompt in default mode.
pub enum PlanFollowUpSubmission {
    Refine { text: String },
    Implement { text: String },
}
pub fn resolve_plan_follow_up_submission(
    draft_text: &str,
    plan_markdown: &str,
) -> PlanFollowUpSubmission {
    let trimmed = draft_text.trim();
    if !trimmed.is_empty() {
        return PlanFollowUpSubmission::Refine {
            text: trimmed.to_owned(),
        };
    }
    PlanFollowUpSubmission::Implement {
        text: build_plan_implementation_prompt(plan_markdown),
    }
}

/// Upstream `buildPlanImplementationThreadTitle`.
pub fn build_plan_implementation_thread_title(plan_markdown: &str) -> String {
    match proposed_plan_title(plan_markdown) {
        Some(title) => format!("Implement {title}"),
        None => "Implement plan".into(),
    }
}

/// Upstream `buildProposedPlanMarkdownFilename` — `{sanitize(title ?? "plan")}.md`.
pub fn build_proposed_plan_markdown_filename(plan_markdown: &str) -> String {
    let title = proposed_plan_title(plan_markdown).unwrap_or_else(|| "plan".into());
    format!("{}.md", sanitize_plan_file_segment(&title))
}

/// Upstream `normalizePlanMarkdownForExport`.
pub fn normalize_plan_markdown_for_export(plan_markdown: &str) -> String {
    format!("{}\n", plan_markdown.trim_end())
}

/// Upstream `hasActionableProposedPlan`.
pub fn has_actionable_proposed_plan(plan: Option<&ProposedPlan>) -> bool {
    plan.is_some_and(|plan| plan.implemented_at_ms.is_none())
}

/// Upstream `findLatestProposedPlan`: prefer a plan from the latest turn, else
/// the most recently updated plan overall (updatedAt, then id).
/// Upstream `findSidebarProposedPlan`: while the implementation turn is
/// unsettled, resolve the plan through the turn's `sourceProposedPlan`
/// reference from the source thread's records; otherwise the active thread's
/// latest unimplemented plan.
pub fn find_sidebar_proposed_plan<'a>(
    proposed_plans: &'a [ProposedPlan],
    source_plans: &'a [ProposedPlan],
    latest_turn: Option<&'a crate::thread::TurnSummary>,
) -> Option<&'a ProposedPlan> {
    if let Some(turn) = latest_turn {
        let settled = turn.finished_at_ms.is_some() || turn.failed;
        if !settled
            && let Some(source) = &turn.source_proposed_plan
            && let Some(plan) = source_plans.iter().find(|plan| plan.id == source.plan_id)
        {
            return Some(plan);
        }
    }
    let unimplemented: Vec<&'a ProposedPlan> = proposed_plans
        .iter()
        .filter(|plan| plan.implemented_at_ms.is_none())
        .collect();
    find_latest_proposed_plan_refs(unimplemented, latest_turn.map(|turn| turn.id.as_str()))
}

fn find_latest_proposed_plan_refs<'a>(
    plans: Vec<&'a ProposedPlan>,
    latest_turn_id: Option<&str>,
) -> Option<&'a ProposedPlan> {
    let by_recency = |plans: &mut Vec<&'a ProposedPlan>| {
        plans.sort_by(|left, right| {
            left.updated_at_ms
                .cmp(&right.updated_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
    };
    if let Some(turn_id) = latest_turn_id {
        let mut matching: Vec<&'a ProposedPlan> = plans
            .iter()
            .copied()
            .filter(|plan| plan.turn_id.as_deref() == Some(turn_id))
            .collect();
        if !matching.is_empty() {
            by_recency(&mut matching);
            return matching.pop();
        }
    }
    let mut plans = plans;
    by_recency(&mut plans);
    plans.pop()
}

pub fn find_latest_proposed_plan<'a>(
    proposed_plans: &'a [ProposedPlan],
    latest_turn_id: Option<&str>,
) -> Option<&'a ProposedPlan> {
    let by_recency = |plans: &mut Vec<&'a ProposedPlan>| {
        plans.sort_by(|left, right| {
            left.updated_at_ms
                .cmp(&right.updated_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
    };
    if let Some(turn_id) = latest_turn_id {
        let mut matching: Vec<&'a ProposedPlan> = proposed_plans
            .iter()
            .filter(|plan| plan.turn_id.as_deref() == Some(turn_id))
            .collect();
        by_recency(&mut matching);
        if let Some(plan) = matching.pop() {
            return Some(plan);
        }
    }
    let mut all: Vec<&'a ProposedPlan> = proposed_plans.iter().collect();
    by_recency(&mut all);
    all.pop()
}

const PROPOSED_PLAN_OPEN: &str = "<proposed_plan>";
const PROPOSED_PLAN_CLOSE: &str = "</proposed_plan>";

/// First `<proposed_plan>…</proposed_plan>` block (case-insensitive tags) →
/// byte range of the inner content, untrimmed. Mirrors the upstream regex
/// `<proposed_plan>\s*([\s\S]*?)\s*<\/proposed_plan>` — non-greedy, first match.
fn first_proposed_plan_block(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_lowercase();
    let open = lower.find(PROPOSED_PLAN_OPEN)?;
    let content_start = open + PROPOSED_PLAN_OPEN.len();
    let content_end = lower[content_start..].find(PROPOSED_PLAN_CLOSE)? + content_start;
    Some((content_start, content_end))
}

/// `/^\s{0,3}#{1,6}\s+/` — up to three leading whitespace chars, then 1-6 `#`,
/// then whitespace. Returns the captured heading body (trimmed).
fn heading_body(line: &str) -> Option<&str> {
    let mut chars = line.char_indices().peekable();
    let mut leading = 0usize;
    while leading < 3 && chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
        chars.next();
        leading += 1;
    }
    let mut hashes = 0usize;
    while chars.peek().is_some_and(|(_, c)| *c == '#') {
        chars.next();
        hashes += 1;
    }
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let (body_start, _) = *chars.peek()?;
    let (_, rest) = line.split_at(body_start);
    rest.chars().next().filter(|c| c.is_whitespace())?;
    Some(rest.trim())
}
fn is_heading_line(line: &str) -> bool {
    heading_body(line).is_some()
}

fn sanitize_plan_file_segment(input: &str) -> String {
    let lower = input.to_lowercase();
    let stripped: String = lower
        .chars()
        .filter(|c| !"`'\".,!?()[]{}".contains(*c))
        .collect();
    let dashed: String = stripped
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let collapsed = dashed
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if collapsed.is_empty() {
        "plan".into()
    } else {
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extract_returns_first_trimmed_block() {
        let text = "Intro.\n<proposed_plan>\n  # Ship it\n  body\n</proposed_plan>\ntail";
        assert_eq!(
            extract_proposed_plan_markdown(text).as_deref(),
            Some("# Ship it\n  body")
        );
        assert_eq!(extract_proposed_plan_markdown("no block"), None);
        assert_eq!(
            extract_proposed_plan_markdown("<proposed_plan>   </proposed_plan>"),
            None
        );
        // Case-insensitive tags, non-greedy first match.
        let two = "<PROPOSED_PLAN>one</PROPOSED_PLAN><proposed_plan>two</proposed_plan>";
        assert_eq!(extract_proposed_plan_markdown(two).as_deref(), Some("one"));
    }
    #[test]
    fn strip_removes_all_blocks_and_trims() {
        let text = "a <proposed_plan>x</proposed_plan> b\n<proposed_plan>y</proposed_plan>\n";
        assert_eq!(strip_proposed_plan_blocks_from_text(text), "a  b");
        assert_eq!(
            strip_proposed_plan_blocks_from_text("<proposed_plan>only</proposed_plan>"),
            ""
        );
    }
    #[test]
    fn title_reads_first_heading() {
        assert_eq!(
            proposed_plan_title("# Title\nbody").as_deref(),
            Some("Title")
        );
        assert_eq!(proposed_plan_title("body"), None);
        assert_eq!(proposed_plan_title("####### seven").as_deref(), None);
        assert_eq!(
            proposed_plan_title("    ## deep").as_deref(),
            None,
            "more than 3 leading spaces is not a heading"
        );
        assert_eq!(
            proposed_plan_title("para\n  ## Later").as_deref(),
            Some("Later")
        );
    }
    #[test]
    fn displayed_strips_title_and_summary() {
        let md = "# Plan title\n\n## Summary\n\nbody line\n- item";
        assert_eq!(strip_displayed_plan_markdown(md), "body line\n- item");
        assert_eq!(strip_displayed_plan_markdown("# Only\nbody"), "body");
    }
    #[test]
    fn collapsed_preview_caps_visible_lines() {
        let md = "# T\n".to_owned() + &(1..=12).map(|i| format!("line {i}\n")).collect::<String>();
        let preview = collapsed_proposed_plan_preview(&md, 3);
        assert_eq!(preview, "line 1\nline 2\nline 3\n\n...");
        let short = collapsed_proposed_plan_preview("# T\nonly", 8);
        assert_eq!(short, "only");
        assert_eq!(
            collapsed_proposed_plan_preview("   ", 8),
            "Plan preview unavailable."
        );
    }
    #[test]
    fn resolve_follow_up_picks_implement_on_empty_draft() {
        match resolve_plan_follow_up_submission("  ", "# P\nsteps") {
            PlanFollowUpSubmission::Implement { text } => {
                assert_eq!(text, "PLEASE IMPLEMENT THIS PLAN:\n# P\nsteps");
            }
            _ => panic!("empty draft must resolve to implementation"),
        }
        match resolve_plan_follow_up_submission("feedback", "# P") {
            PlanFollowUpSubmission::Refine { text } => assert_eq!(text, "feedback"),
            _ => panic!("non-empty draft must stay a plan-mode refinement"),
        }
    }
    #[test]
    fn titles_and_filenames() {
        assert_eq!(
            build_plan_implementation_thread_title("body"),
            "Implement plan"
        );
        assert_eq!(
            build_plan_implementation_thread_title("# Ship It!\nsteps"),
            "Implement Ship It!"
        );
        assert_eq!(
            build_proposed_plan_markdown_filename("# Ship It!\nsteps"),
            "ship-it.md"
        );
        assert_eq!(build_proposed_plan_markdown_filename("body"), "plan.md");
        assert_eq!(normalize_plan_markdown_for_export("a\n\n"), "a\n");
    }
    #[test]
    fn latest_plan_prefers_matching_turn_then_recency() {
        let plan = |id: &str, turn: &str, updated: i64| ProposedPlan {
            id: id.into(),
            turn_id: Some(turn.into()),
            plan_markdown: id.into(),
            implemented_at_ms: None,
            implementation_thread_id: None,
            created_at_ms: updated,
            updated_at_ms: updated,
        };
        let plans = vec![
            plan("old", "t1", 1),
            plan("turn-latest", "t2", 2),
            plan("newer-other-turn", "t1", 9),
        ];
        assert_eq!(
            find_latest_proposed_plan(&plans, Some("t2")).map(|p| p.id.as_str()),
            Some("turn-latest")
        );
        assert_eq!(
            find_latest_proposed_plan(&plans, Some("t9")).map(|p| p.id.as_str()),
            Some("newer-other-turn")
        );
        assert_eq!(find_latest_proposed_plan(&[], Some("t1")), None);
        let mut done = plans.clone();
        done[2].implemented_at_ms = Some(5);
        assert!(has_actionable_proposed_plan(done.first()));
        assert!(!has_actionable_proposed_plan(done.get(2)));
    }
}
