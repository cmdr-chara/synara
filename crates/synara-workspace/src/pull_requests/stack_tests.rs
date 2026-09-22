use super::*;
fn repo() -> GithubRepository {
    GithubRepository::parse("https://github.com/owner/project").unwrap()
}
fn pr(number: u64, head: &str, base: &str) -> Value {
    json!({"number":number,"title":format!("PR {number}"),"state":"open","draft":false,"merged":false,
        "head":{"ref":head,"sha":format!("{number:040x}"),"repo":{"full_name":"owner/project"}},
        "base":{"ref":base,"sha":"a".repeat(40),"repo":{"full_name":"owner/project"}}})
}
fn ready(mut review: StackReview) -> StackReview {
    for row in &mut review.rows {
        row.blocked = None;
    }
    review
}
#[test]
fn stack_order_and_selected_prefix_are_deterministic_not_api_order() {
    let data = vec![
        pr(4, "four", "two"),
        pr(3, "three", "one"),
        pr(1, "one", "main"),
        pr(2, "two", "one"),
        pr(8, "other", "main"),
    ];
    let review = graph(&repo(), 4, &data).unwrap();
    assert_eq!(
        review
            .rows()
            .iter()
            .map(StackRow::number)
            .collect::<Vec<_>>(),
        vec![1, 2, 4, 3]
    );
    assert_eq!(
        review
            .prefix()
            .unwrap()
            .iter()
            .map(|r| r.number())
            .collect::<Vec<_>>(),
        vec![1, 2, 4]
    );
    assert_eq!(review.rows()[2].parent, Some(2));
    assert_eq!(review.rows()[2].depth, 2);
    let reverse: Vec<_> = data.into_iter().rev().collect();
    assert_eq!(
        graph(&repo(), 4, &reverse)
            .unwrap()
            .rows()
            .iter()
            .map(StackRow::number)
            .collect::<Vec<_>>(),
        vec![1, 2, 4, 3]
    );
}
#[test]
fn stack_rejects_cycles_duplicates_missing_or_ambiguous_identity() {
    for data in [
        vec![pr(1, "one", "two"), pr(2, "two", "one")],
        vec![pr(1, "one", "main"), pr(1, "two", "one")],
        vec![pr(1, "one", "main"), pr(2, "one", "main")],
        vec![],
    ] {
        assert!(graph(&repo(), 1, &data).is_err());
    }
    let mut missing = pr(1, "one", "main");
    missing["head"]["repo"] = Value::Null;
    assert!(graph(&repo(), 1, &[missing]).is_err());
    let mut bad = pr(1, "one", "main");
    bad["head"]["sha"] = json!("branch");
    assert!(graph(&repo(), 1, &[bad]).is_err());
}
#[test]
fn stack_does_not_guess_fork_parent_from_matching_branch_name() {
    let mut fork = pr(1, "one", "main");
    fork["head"]["repo"]["full_name"] = json!("other/project");
    let review = graph(&repo(), 2, &[fork, pr(2, "two", "one")]).unwrap();
    assert_eq!(review.rows.len(), 1);
    assert_eq!(review.rows[0].parent, None);
}
#[test]
fn stack_scope_is_canonical_and_cross_repository_base_is_rejected() {
    let mut v = pr(1, "one", "main");
    v["base"]["repo"]["full_name"] = json!("other/project");
    assert!(graph(&repo(), 1, &[v]).is_err());
    let mut v = pr(1, "one", "main");
    v["head"]["repo"]["full_name"] = json!("Owner/Project");
    assert_eq!(
        graph(&repo(), 1, &[v]).unwrap().rows[0].identity.head_repo,
        "owner/project"
    );
}
#[test]
fn stack_limits_are_fail_closed_not_truncation() {
    let data: Vec<_> = (1..=17)
        .map(|i| {
            pr(
                i,
                &format!("branch{i}"),
                if i == 1 { "main" } else { "branch1" },
            )
        })
        .collect();
    assert!(graph(&repo(), 1, &data).unwrap_err().contains("16"));
    let mut v = pr(1, "one", "main");
    v["title"] = json!("x".repeat(1025));
    assert!(graph(&repo(), 1, &[v]).is_err());
    let mut v = pr(1, "one", "main");
    v["head"]["ref"] = json!("one\nmain");
    assert!(graph(&repo(), 1, &[v]).is_err());
}
#[test]
fn stack_unknown_failed_pending_and_incomplete_checks_block() {
    let checks =
        json!({"total_count":1,"check_runs":[{"status":"completed","conclusion":"success"}]});
    let status = json!({"total_count":1,"state":"success","statuses":[{"state":"success"}]});
    assert!(checks_ready(&checks, &status).is_ok());
    for conclusion in ["failure", "cancelled", "timed_out", "action_required", ""] {
        let mut changed = checks.clone();
        changed["check_runs"][0]["conclusion"] = json!(conclusion);
        assert!(checks_ready(&changed, &status).is_err());
    }
    let mut changed = checks.clone();
    changed["total_count"] = json!(101);
    assert!(checks_ready(&changed, &status).is_err());
    let mut changed = checks.clone();
    changed["check_runs"][0]["status"] = json!("in_progress");
    assert!(checks_ready(&changed, &status).is_err());
    assert!(checks_ready(&Value::Null, &status).is_err());
    assert!(
        checks_ready(
            &checks,
            &json!({"total_count":0,"state":"pending","statuses":[]})
        )
        .is_ok()
    );
    assert!(
        checks_ready(
            &checks,
            &json!({"total_count":1,"state":"pending","statuses":[{"state":"pending"}]})
        )
        .is_err()
    );
}
#[test]
fn stack_selection_never_includes_unselected_siblings_or_inherits_readiness() {
    let mut review = ready(
        graph(
            &repo(),
            2,
            &[
                pr(1, "one", "main"),
                pr(2, "two", "one"),
                pr(3, "three", "one"),
            ],
        )
        .unwrap(),
    );
    review.rows[2].blocked = Some("failed".into());
    assert!(review.ready());
    review.select(3).unwrap();
    assert!(!review.ready());
    assert!(review.select(9).is_err());
    assert_eq!(review.selected(), 3);
}
#[tokio::test]
async fn stack_confirmation_scope_and_cancel_gate_before_process_spawn() {
    let service = PullRequests::new(PathBuf::from("/not-a-root"));
    let review = ready(graph(&repo(), 1, &[pr(1, "one", "main")]).unwrap());
    assert!(
        service
            .merge_stack_prefix(&repo(), review.clone(), false, Default::default())
            .await
            .unwrap_err()
            .contains("confirmation")
    );
    let other = GithubRepository::parse("https://github.com/other/project").unwrap();
    assert!(
        service
            .merge_stack_prefix(&other, review.clone(), true, Default::default())
            .await
            .unwrap_err()
            .contains("scope")
    );
    let cancel = PullRequestCancellation::new();
    cancel.cancel();
    assert!(
        service
            .merge_stack_prefix(&repo(), review, true, cancel)
            .await
            .unwrap_err()
            .contains("before any")
    );
}
#[test]
fn stack_head_fences_include_repository_branch_and_commit_not_only_number() {
    let a = parse_identity(&repo(), &pr(1, "one", "main")).unwrap();
    for key in ["sha", "ref"] {
        let mut v = pr(1, "one", "main");
        v["head"][key] = json!(if key == "sha" {
            "b".repeat(40)
        } else {
            "renamed".into()
        });
        assert!(!same_head(&a, &parse_identity(&repo(), &v).unwrap()));
    }
    let mut v = pr(1, "one", "main");
    v["head"]["repo"]["full_name"] = json!("fork/project");
    assert!(!same_head(&a, &parse_identity(&repo(), &v).unwrap()));
}
#[test]
fn stack_progress_reports_partial_writes_and_no_automatic_rollback() {
    let report = StackProgress {
        merged: vec![1],
        retargeted: vec![2],
        stopped: Some(ambiguous_write("Cancelled".into())),
    };
    let text = report.summary();
    for expected in ["[1]", "[2]", "Cancelled", "No automatic retry or rollback"] {
        assert!(text.contains(expected));
    }
}
