# ACP transport regression matrix

Roadmap owner: B5. This matrix describes executable coverage, not a result claim.
Initial verification state: pending native backend and Linux workspace CI.

| Contract | Deciding regression |
| --- | --- |
| Invalid JSON-RPC version, envelope, params, ID and error shape | `rpc_protocol_tests::invalid_envelopes_fail_closed` |
| Stdout contamination, malformed JSON and invalid UTF-8 | `stdout_contamination_malformed_json_and_invalid_utf8_fail_closed` |
| Oversized unterminated frame | `oversized_unterminated_stdout_is_bounded` |
| Partial UTF-8 reads, CRLF and multiple messages | `fragmented_utf8_and_multiple_frames_preserve_notification_order` |
| EOF with an incomplete frame | `incomplete_frame_at_eof_is_not_accepted_or_silently_discarded` |
| Duplicate incoming ownership | `duplicate_incoming_request_ids_are_rejected` |
| Numeric versus string request IDs | `numeric_and_string_callback_ids_have_distinct_ownership` |
| Out-of-order, unknown and late/duplicate response isolation | `out_of_order_late_and_unknown_responses_cannot_complete_other_requests` |
| Incoming notification queue backpressure | `incoming_notification_backpressure_fails_instead_of_growing_unbounded` |
| Independent incoming callback ownership cap | `incoming_callback_ownership_has_an_independent_limit` |
| Pending request cap and recovery | `pending_request_limit_does_not_disconnect_healthy_requests` |
| Unsupported callback error encoding and safe messages | `unsupported_callback_replies_use_protocol_errors_not_host_details` |
| Stderr flood, bounded redaction and independent stdout traffic | `stderr_floods_remain_bounded_redacted_and_independent_of_protocol_output` |
| Last-owner teardown with a writer that never finishes shutdown | `last_owner_drop_releases_all_pipes_even_when_shutdown_never_completes` |
| Timeout and abort before queued request transmission | `rpc::outbound::tests::{timed_out,aborted}_queued_requests_never_reach_agent_stdin` |
| Cancellation during partial transmission | `cancelling_a_partial_frame_closes_stream_without_sending_siblings` |
| Complete transmission does not disconnect sibling sessions | `cancelling_a_delivered_frame_keeps_sibling_sessions_usable` |
| Oversized output serialization releases reservations | `oversized_serialization_releases_its_reservation_without_enqueuing` |
| Aggregate output and serialization byte budget | `serialized_frames_and_blocked_producers_share_one_byte_budget` |
| Queue closure, pre-cancelled input and shutdown while waiting for capacity | `closed_writer_queue_does_not_leak_a_serialization_reservation`, `already_cancelled_requests_do_not_allocate_or_enqueue_frames`, `connection_shutdown_wakes_a_blocked_budget_waiter` |
| Capacity growth, delimiter allocation and payload-free Debug | `rpc::outbound::allocation_tests` |
| Existing callback expiry, first-failure ownership and sibling lifetime isolation | `rpc::lifecycle_tests` |
| Actual child-agent crash, malformed output, prompt timeout and cancellation recovery | `tests/process_integration.rs` |

Run focused transport coverage with:

```sh
cargo +1.98.1 test --locked -p synara-acp rpc
cargo +1.98.1 test --locked -p synara-acp --test process_integration
```

The deterministic transport tests do not install or authenticate vendor agents.
In-process pipes are deliberately separate evidence from the actual child-agent
fixture integration, Linux SSH verification, native desktop smoke and vendor
compatibility probes. All applicable existing checks must still pass.

The output byte budget now charges allocated vector capacity rather than logical
length alone, including spare capacity and the terminating newline. Growth is
geometric but capped, avoiding repeated exact-size reallocation for heavily
escaped strings or many small serialization fragments. Protocol payload bytes
are excluded from Frame Debug output.
