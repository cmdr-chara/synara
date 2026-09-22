use super::*;
fn repo() -> GithubRepository {
    GithubRepository::parse("https://github.com/owner/project").unwrap()
}
fn node() -> Value {
    json!({"id":"PRRT_1","isResolved":false,"isOutdated":false,"path":"src/lib.rs","diffSide":"RIGHT","startDiffSide":null,"subjectType":"LINE","line":4,"startLine":null,"originalLine":3,"originalStartLine":null,
        "comments":{"totalCount":1,"pageInfo":{"hasNextPage":false},"nodes":[{"id":"PRRC_1","url":"https://github.com/owner/project/pull/7#discussion_r12","body":"Review this branch","path":"src/lib.rs","diffHunk":"@@ -1,1 +1,2 @@\n+new","updatedAt":"2026-09-22T00:00:00Z","originalCommit":{"oid":"b".repeat(40)},"commit":{"oid":"a".repeat(40)},"author":{"login":"reviewer"}}]}})
}
fn page(nodes: Vec<Value>) -> Value {
    json!({"data":{"repository":{"nameWithOwner":"owner/project","pullRequest":{"number":7,"state":"OPEN","headRefOid":"a".repeat(40),"reviewThreads":{"totalCount":nodes.len(),"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":nodes}}}}})
}
fn snapshot() -> FixSnapshot {
    let page = parse_page(page(vec![node()]), &repo(), 7, &"a".repeat(40)).unwrap();
    FixSnapshot {
        version: 1,
        repository: repo(),
        number: 7,
        head_sha: "a".repeat(40),
        collected_ms: 1,
        threads: page.threads,
    }
}
#[test]
fn unresolved_comments_retain_exact_identity_location_context_and_commit() {
    let mut resolved = node();
    resolved["id"] = json!("resolved");
    resolved["isResolved"] = json!(true);
    let parsed = parse_page(page(vec![resolved, node()]), &repo(), 7, &"a".repeat(40)).unwrap();
    assert_eq!(parsed.ids.len(), 2);
    assert_eq!(parsed.threads.len(), 1);
    let thread = &parsed.threads[0];
    assert_eq!(thread.id, "PRRT_1");
    assert_eq!(thread.line, Some(4));
    assert_eq!(thread.original_line, Some(3));
    assert_eq!(thread.comments[0].id, "PRRC_1");
    assert_eq!(
        thread.comments[0].original_commit.as_ref().unwrap().oid,
        "b".repeat(40)
    );
    let text = snapshot().instruction_set().unwrap();
    assert!(text.contains("untrusted external review material"));
    assert!(text.contains("Stop on a mismatch"));
    assert!(text.contains("discussion_r12"));
}
#[test]
fn incomplete_graphql_and_truncated_threads_fail_closed() {
    let mut response = page(vec![node()]);
    response["errors"] = json!([{"message":"sensitive diagnostics"}]);
    assert!(
        parse_page(response, &repo(), 7, &"a".repeat(40))
            .unwrap_err()
            .contains("incomplete")
    );
    let mut n = node();
    n["comments"]["pageInfo"]["hasNextPage"] = json!(true);
    assert!(parse_page(page(vec![n]), &repo(), 7, &"a".repeat(40)).is_err());
    let mut n = node();
    n["isResolved"] = Value::Null;
    assert!(parse_page(page(vec![n]), &repo(), 7, &"a".repeat(40)).is_err());
    assert!(parse_page(page(vec![Value::Null]), &repo(), 7, &"a".repeat(40)).is_err());
}
#[test]
fn head_repository_and_open_state_are_not_guessed() {
    assert!(parse_page(page(vec![node()]), &repo(), 7, &"b".repeat(40)).is_err());
    assert!(parse_page(page(vec![node()]), &repo(), 8, &"a".repeat(40)).is_err());
    let mut response = page(vec![node()]);
    response["data"]["repository"]["pullRequest"]["state"] = json!("CLOSED");
    assert!(parse_page(response, &repo(), 7, &"a".repeat(40)).is_err());
}
#[test]
fn fingerprint_detects_edits_locations_new_replies_and_resolution_not_clock() {
    let source = snapshot();
    let hash = source.fingerprint().unwrap();
    let mut later = source.clone();
    later.collected_ms += 100;
    assert_eq!(later.fingerprint().unwrap(), hash);
    later.threads[0].comments[0].body.push('!');
    assert_ne!(later.fingerprint().unwrap(), hash);
    let mut moved = source.clone();
    moved.threads[0].line = Some(8);
    assert_ne!(moved.fingerprint().unwrap(), hash);
    let mut resolved = source.clone();
    resolved.threads.clear();
    assert_ne!(resolved.fingerprint().unwrap(), hash);
    let mut reply = source.clone();
    let mut comment = reply.threads[0].comments[0].clone();
    comment.id = "PRRC_2".into();
    reply.threads[0].comments.push(comment);
    assert_ne!(reply.fingerprint().unwrap(), hash);
}
#[test]
fn duplicates_unsafe_paths_links_and_oversized_bodies_are_rejected() {
    let mut duplicate = snapshot();
    duplicate.threads.push(duplicate.threads[0].clone());
    assert!(duplicate.validate().is_err());
    for path in ["../secret", "/etc/passwd", "a\\b", "C:/private", "a/./b"] {
        let mut s = snapshot();
        s.threads[0].path = path.into();
        assert!(s.validate().is_err());
    }
    for url in [
        "https://evil.invalid/owner/project/pull/7#discussion_r12",
        "https://github.com/owner/other/pull/7#discussion_r12",
        "https://github.com/owner/project/pull/8#discussion_r12",
    ] {
        let mut s = snapshot();
        s.threads[0].comments[0].url = url.into();
        assert!(s.validate().is_err());
    }
    let mut s = snapshot();
    s.threads[0].comments[0].body = "x".repeat(16385);
    assert!(s.validate().is_err());
}
#[test]
fn unknown_and_outdated_line_numbers_remain_explicitly_unknown() {
    let mut s = snapshot();
    s.threads[0].is_outdated = true;
    s.threads[0].line = None;
    s.threads[0].start_line = None;
    s.validate().unwrap();
    let text = s.instruction_set().unwrap();
    assert!(text.contains("\"line\": null"));
    assert!(text.contains("\"originalLine\": 3"));
    assert!(text.contains("\"isOutdated\": true"));
}
#[test]
fn graphql_uses_only_queries_and_json_variables_not_shell_interpolation() {
    assert!(QUERY.starts_with("query "));
    assert!(!QUERY.contains("mutation"));
    let body = json!({"query":QUERY,"variables":{"owner":"owner","name":"project","number":7,"after":"literal-$(unsafe)"}});
    let (launch, stdin) = request_plan("POST", "graphql", Some(body)).unwrap();
    assert_eq!(launch.command, PathBuf::from("gh"));
    assert!(!launch.args.iter().any(|arg| arg.contains("$(")));
    assert!(
        String::from_utf8(stdin.unwrap())
            .unwrap()
            .contains("literal-$(unsafe)")
    );
}
#[tokio::test]
async fn cancellation_happens_before_starting_a_cli_process() {
    let service = PullRequests::new(PathBuf::from("/not-a-directory"));
    let cancel = PullRequestCancellation::new();
    cancel.cancel();
    assert_eq!(
        service
            .collect_unresolved_reviews(&repo(), 7, &"a".repeat(40), cancel)
            .await
            .unwrap_err(),
        "Cancelled"
    );
}

#[test]
fn empty_or_overcounted_continuation_pages_fail_without_following_a_cursor() {
    let mut response = page(vec![]);
    response["data"]["repository"]["pullRequest"]["reviewThreads"]["pageInfo"] =
        json!({"hasNextPage":true,"endCursor":"cursor"});
    assert!(parse_page(response, &repo(), 7, &"a".repeat(40)).is_err());
    let mut response = page(vec![node()]);
    response["data"]["repository"]["pullRequest"]["reviewThreads"]["totalCount"] = json!(0);
    assert!(parse_page(response, &repo(), 7, &"a".repeat(40)).is_err());
}
#[test]
fn snapshot_limits_and_invalid_line_ranges_fail_before_draft_preparation() {
    let mut s = snapshot();
    s.threads[0].start_line = Some(5);
    assert!(s.instruction_set().is_err());
    s.threads[0].start_line = None;
    s.threads[0].line = Some(0);
    assert!(s.instruction_set().is_err());
    let mut s = snapshot();
    s.threads = (0..MAX_FIX_THREADS + 1)
        .map(|i| {
            let mut thread = s.threads[0].clone();
            thread.id = format!("thread-{i}");
            thread.comments[0].id = format!("comment-{i}");
            thread
        })
        .collect();
    assert!(s.validate().is_err());
    let mut s = snapshot();
    s.threads.clear();
    assert!(s.instruction_set().unwrap_err().contains("No unresolved"));
}
