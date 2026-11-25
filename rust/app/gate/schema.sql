CREATE TABLE IF NOT EXISTS "effect_sql_migrations" (
  migration_id integer PRIMARY KEY NOT NULL,
  created_at datetime NOT NULL DEFAULT current_timestamp,
  name VARCHAR(255) NOT NULL
);
CREATE TABLE orchestration_events (
      sequence INTEGER PRIMARY KEY AUTOINCREMENT,
      event_id TEXT NOT NULL UNIQUE,
      aggregate_kind TEXT NOT NULL,
      stream_id TEXT NOT NULL,
      stream_version INTEGER NOT NULL,
      event_type TEXT NOT NULL,
      occurred_at TEXT NOT NULL,
      command_id TEXT,
      causation_event_id TEXT,
      correlation_id TEXT,
      actor_kind TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      metadata_json TEXT NOT NULL
    );
CREATE UNIQUE INDEX idx_orch_events_stream_version
    ON orchestration_events(aggregate_kind, stream_id, stream_version)
  ;
CREATE INDEX idx_orch_events_stream_sequence
    ON orchestration_events(aggregate_kind, stream_id, sequence)
  ;
CREATE TABLE orchestration_command_receipts (
      command_id TEXT PRIMARY KEY,
      aggregate_kind TEXT NOT NULL,
      aggregate_id TEXT NOT NULL,
      accepted_at TEXT NOT NULL,
      result_sequence INTEGER NOT NULL,
      status TEXT NOT NULL,
      error TEXT
    , fingerprint_version INTEGER
      CHECK (fingerprint_version IS NULL OR fingerprint_version > 0), command_fingerprint TEXT
      CHECK (
        command_fingerprint IS NULL OR (
          length(command_fingerprint) = 64 AND
          command_fingerprint NOT GLOB '*[^0-9a-f]*'
        )
      ));
CREATE INDEX idx_orch_command_receipts_aggregate
    ON orchestration_command_receipts(aggregate_kind, aggregate_id)
  ;
CREATE INDEX idx_orch_command_receipts_sequence
    ON orchestration_command_receipts(result_sequence)
  ;
CREATE TABLE checkpoint_diff_blobs (
      thread_id TEXT NOT NULL,
      from_turn_count INTEGER NOT NULL,
      to_turn_count INTEGER NOT NULL,
      diff TEXT NOT NULL,
      created_at TEXT NOT NULL,
      UNIQUE (thread_id, from_turn_count, to_turn_count)
    );
CREATE INDEX idx_checkpoint_diff_blobs_thread_to_turn
    ON checkpoint_diff_blobs(thread_id, to_turn_count)
  ;
CREATE TABLE provider_session_runtime (
      thread_id TEXT PRIMARY KEY,
      provider_name TEXT NOT NULL,
      adapter_key TEXT NOT NULL,
      runtime_mode TEXT NOT NULL DEFAULT 'full-access',
      status TEXT NOT NULL,
      last_seen_at TEXT NOT NULL,
      resume_cursor_json TEXT,
      runtime_payload_json TEXT
    , lifecycle_generation TEXT NOT NULL DEFAULT 'legacy', provider_instance_id TEXT);
CREATE INDEX idx_provider_session_runtime_status
    ON provider_session_runtime(status)
  ;
CREATE INDEX idx_provider_session_runtime_provider
    ON provider_session_runtime(provider_name)
  ;
CREATE TABLE projection_projects (
      project_id TEXT PRIMARY KEY,
      kind TEXT NOT NULL DEFAULT 'project',
      title TEXT NOT NULL,
      workspace_root TEXT NOT NULL,
      scripts_json TEXT NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      deleted_at TEXT
    , default_model_selection_json TEXT, is_pinned INTEGER NOT NULL DEFAULT 0, space_id TEXT, additional_folders_json TEXT NOT NULL DEFAULT '[]');
CREATE TABLE projection_threads (
      thread_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      title TEXT NOT NULL,
      branch TEXT,
      worktree_path TEXT,
      latest_turn_id TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      deleted_at TEXT
    , runtime_mode TEXT NOT NULL DEFAULT 'full-access', interaction_mode TEXT NOT NULL DEFAULT 'default', model_selection_json TEXT, handoff_json TEXT, env_mode TEXT NOT NULL DEFAULT 'local', fork_source_thread_id TEXT, associated_worktree_path TEXT, associated_worktree_branch TEXT, associated_worktree_ref TEXT, archived_at TEXT, parent_thread_id TEXT, subagent_agent_id TEXT, subagent_nickname TEXT, subagent_role TEXT, latest_user_message_at TEXT, pending_approval_count INTEGER NOT NULL DEFAULT 0, pending_user_input_count INTEGER NOT NULL DEFAULT 0, has_actionable_proposed_plan INTEGER NOT NULL DEFAULT 0, last_known_pr_json TEXT, create_branch_flow_completed INTEGER NOT NULL DEFAULT 0, sidechat_source_thread_id TEXT, is_pinned INTEGER NOT NULL DEFAULT 0, pinned_messages_json TEXT, notes TEXT, creation_source TEXT, source_thread_id TEXT, source_turn_id TEXT, gateway_operation_id TEXT, gateway_operation_index INTEGER, working_directory TEXT, settled_at TEXT, goal TEXT, goal_started_at TEXT, goal_paused_at TEXT, goal_achievements_json TEXT, sidechat_last_activity_at TEXT, sidechat_expired_at TEXT, claude_cache_review_json TEXT, latest_human_message_at TEXT, sidechat_context_json TEXT, snoozed_until TEXT, snooze_reminder_at TEXT, fork_source_message_id TEXT, usage_limit_recovery_source_json TEXT, usage_limit_recovery_json TEXT, usage_limit_recovery_cancelled_at TEXT, usage_limit_recovery_cancelled_turn_id TEXT, usage_limit_recovery_last_reset_at TEXT, pull_requests_json TEXT);
CREATE TABLE projection_thread_activities (
      activity_id TEXT PRIMARY KEY,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      tone TEXT NOT NULL,
      kind TEXT NOT NULL,
      summary TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      created_at TEXT NOT NULL
    , sequence INTEGER);
CREATE TABLE projection_thread_sessions (
      thread_id TEXT PRIMARY KEY,
      status TEXT NOT NULL,
      provider_name TEXT,
      provider_session_id TEXT,
      provider_thread_id TEXT,
      active_turn_id TEXT,
      last_error TEXT,
      updated_at TEXT NOT NULL
    , runtime_mode TEXT NOT NULL DEFAULT 'full-access', last_activity_at TEXT, last_progress_at TEXT, provider_instance_id TEXT);
CREATE TABLE projection_turns (
      row_id INTEGER PRIMARY KEY AUTOINCREMENT,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      pending_message_id TEXT,
      assistant_message_id TEXT,
      state TEXT NOT NULL,
      requested_at TEXT NOT NULL,
      started_at TEXT,
      completed_at TEXT,
      checkpoint_turn_count INTEGER,
      checkpoint_ref TEXT,
      checkpoint_status TEXT,
      checkpoint_files_json TEXT NOT NULL, source_proposed_plan_thread_id TEXT, source_proposed_plan_id TEXT, started_without_git_workspace INTEGER NOT NULL DEFAULT 0,
      UNIQUE (thread_id, turn_id),
      UNIQUE (thread_id, checkpoint_turn_count)
    );
CREATE TABLE projection_state (
      projector TEXT PRIMARY KEY,
      last_applied_sequence INTEGER NOT NULL,
      updated_at TEXT NOT NULL
    );
CREATE INDEX idx_projection_projects_updated_at
    ON projection_projects(updated_at)
  ;
CREATE INDEX idx_projection_threads_project_id
    ON projection_threads(project_id)
  ;
CREATE INDEX idx_projection_thread_activities_thread_created
    ON projection_thread_activities(thread_id, created_at)
  ;
CREATE INDEX idx_projection_thread_sessions_provider_session
    ON projection_thread_sessions(provider_session_id)
  ;
CREATE INDEX idx_projection_turns_thread_requested
    ON projection_turns(thread_id, requested_at)
  ;
CREATE INDEX idx_projection_turns_thread_checkpoint_completed
    ON projection_turns(thread_id, checkpoint_turn_count, completed_at)
  ;
CREATE INDEX idx_projection_thread_activities_thread_sequence
    ON projection_thread_activities(thread_id, sequence)
  ;
CREATE TABLE projection_thread_proposed_plans (
      plan_id TEXT PRIMARY KEY,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      plan_markdown TEXT NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    , implemented_at TEXT, implementation_thread_id TEXT);
CREATE INDEX idx_projection_thread_proposed_plans_thread_created
    ON projection_thread_proposed_plans(thread_id, created_at)
  ;
CREATE INDEX idx_projection_threads_parent_thread_id
    ON projection_threads(parent_thread_id)
  ;
CREATE TABLE auth_pairing_links (
      id TEXT PRIMARY KEY,
      credential TEXT NOT NULL UNIQUE,
      method TEXT NOT NULL,
      role TEXT NOT NULL,
      subject TEXT NOT NULL,
      label TEXT,
      created_at TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      consumed_at TEXT,
      revoked_at TEXT
    , purpose TEXT NOT NULL DEFAULT 'local-session', remote_metadata TEXT, pending_device_jkt TEXT, pending_device_key TEXT, pending_device_label TEXT, access_origin TEXT
      CHECK (access_origin IS NULL OR access_origin IN ('local', 'remote')));
CREATE INDEX idx_auth_pairing_links_active
    ON auth_pairing_links(revoked_at, consumed_at, expires_at)
  ;
CREATE TABLE auth_sessions (
      session_id TEXT PRIMARY KEY,
      subject TEXT NOT NULL,
      role TEXT NOT NULL,
      method TEXT NOT NULL,
      client_label TEXT,
      client_ip_address TEXT,
      client_user_agent TEXT,
      client_device_type TEXT NOT NULL DEFAULT 'unknown',
      client_os TEXT,
      client_browser TEXT,
      issued_at TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      last_connected_at TEXT,
      revoked_at TEXT
    , access_origin TEXT
      CHECK (access_origin IS NULL OR access_origin IN ('local', 'remote')));
CREATE INDEX idx_auth_sessions_active
    ON auth_sessions(revoked_at, expires_at, issued_at)
  ;
CREATE INDEX idx_projection_thread_activities_thread_rank_desc
    ON projection_thread_activities(
      thread_id,
      (CASE WHEN sequence IS NULL THEN 0 ELSE 1 END) DESC,
      sequence DESC,
      created_at DESC,
      activity_id DESC
    )
  ;
CREATE INDEX idx_orchestration_events_profile_turn_events
    ON orchestration_events(event_type, stream_id)
  ;
CREATE INDEX idx_projection_thread_activities_profile_token_activity
    ON projection_thread_activities(kind, thread_id, sequence, created_at, activity_id)
  ;
CREATE TABLE automation_definitions (
      automation_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      source_thread_id TEXT,
      name TEXT NOT NULL,
      prompt TEXT NOT NULL,
      schedule_json TEXT NOT NULL,
      enabled INTEGER NOT NULL,
      next_run_at TEXT,
      model_selection_json TEXT NOT NULL,
      provider_options_json TEXT,
      runtime_mode TEXT NOT NULL,
      interaction_mode TEXT NOT NULL,
      worktree_mode TEXT NOT NULL,
      mode TEXT NOT NULL,
      target_thread_id TEXT,
      max_iterations INTEGER,
      stop_on_error INTEGER NOT NULL,
      completion_policy_json TEXT NOT NULL DEFAULT '{"type":"none"}',
      completion_policy_version INTEGER NOT NULL DEFAULT 0,
      completion_policy_updated_at TEXT,
      minimum_interval_seconds INTEGER NOT NULL,
      max_runtime_seconds INTEGER,
      retry_policy_json TEXT NOT NULL,
      misfire_policy TEXT NOT NULL,
      acknowledged_risks_json TEXT NOT NULL,
      iteration_count INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      archived_at TEXT
    , proposal_state TEXT
      CHECK (proposal_state IS NULL OR proposal_state IN ('pending', 'accepted', 'dismissed')), heartbeat_cooldown_seconds INTEGER NOT NULL DEFAULT 60, notification_policy TEXT
      CHECK (
        notification_policy IS NULL
        OR notification_policy IN ('all', 'failed-runs-only')
      ), stop_after_consecutive_failures INTEGER
      CHECK (
        stop_after_consecutive_failures IS NULL
        OR stop_after_consecutive_failures >= 1
      ), consecutive_failure_count INTEGER NOT NULL DEFAULT 0, disabled_reason TEXT
      CHECK (
        disabled_reason IS NULL
        OR disabled_reason IN ('failures', 'max-iterations', 'completion', 'schedule', 'user')
      ), disabled_at TEXT, managed_by_project INTEGER NOT NULL DEFAULT 0);
CREATE TABLE automation_runs (
      run_id TEXT PRIMARY KEY,
      automation_id TEXT NOT NULL,
      project_id TEXT NOT NULL,
      thread_id TEXT,
      turn_id TEXT,
      trigger_type TEXT NOT NULL,
      status TEXT NOT NULL,
      scheduled_for TEXT NOT NULL,
      claimed_by TEXT,
      claimed_at TEXT,
      lease_expires_at TEXT,
      started_at TEXT,
      finished_at TEXT,
      thread_create_command_id TEXT,
      turn_start_command_id TEXT,
      message_id TEXT,
      error TEXT,
      result_json TEXT,
      permission_snapshot_json TEXT NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL, deferred_until TEXT,
      FOREIGN KEY (automation_id) REFERENCES automation_definitions(automation_id)
    );
CREATE TABLE automation_scheduler_leases (
      lease_key TEXT PRIMARY KEY,
      owner_id TEXT NOT NULL,
      acquired_at TEXT NOT NULL,
      heartbeat_at TEXT NOT NULL,
      lease_expires_at TEXT NOT NULL
    );
CREATE INDEX idx_automation_definitions_due
    ON automation_definitions (enabled, archived_at, next_run_at, automation_id)
  ;
CREATE UNIQUE INDEX idx_automation_runs_unique_occurrence
    ON automation_runs (automation_id, scheduled_for)
    WHERE trigger_type = 'scheduled'
  ;
CREATE INDEX idx_automation_runs_recovery
    ON automation_runs (status, lease_expires_at)
  ;
CREATE INDEX idx_automation_runs_history
    ON automation_runs (automation_id, scheduled_for DESC, run_id DESC)
  ;
CREATE INDEX idx_automation_runs_project
    ON automation_runs (project_id, scheduled_for DESC, run_id DESC)
  ;
CREATE INDEX idx_automation_runs_thread
    ON automation_runs (thread_id, created_at DESC)
  ;
CREATE INDEX idx_automation_runs_completion_eval
    ON automation_runs (finished_at, run_id)
    WHERE status = 'succeeded' AND finished_at IS NOT NULL
  ;
CREATE VIEW automation_pending_completion_evaluations AS
    SELECT
      runs.run_id,
      runs.automation_id,
      runs.thread_id,
      runs.finished_at
    FROM automation_runs runs
    INNER JOIN automation_definitions definitions
      ON definitions.automation_id = runs.automation_id
    WHERE runs.status = 'succeeded'
      AND definitions.enabled = 1
      AND definitions.archived_at IS NULL
      AND definitions.mode = 'heartbeat'
      AND json_extract(definitions.completion_policy_json, '$.type') = 'ai-evaluated'
      AND runs.finished_at IS NOT NULL
      AND (
        json_extract(runs.permission_snapshot_json, '$.completionPolicyVersion') =
          definitions.completion_policy_version
        OR (
          json_type(runs.permission_snapshot_json, '$.completionPolicyVersion') IS NULL
          AND COALESCE(runs.started_at, runs.created_at) >
            COALESCE(
              definitions.completion_policy_updated_at,
              definitions.updated_at,
              definitions.created_at,
              '1970-01-01T00:00:00.000Z'
            )
        )
      )
      AND (
        runs.result_json IS NULL
        OR json_type(runs.result_json, '$.completionEvaluation') IS NULL
      )
/* automation_pending_completion_evaluations(run_id,automation_id,thread_id,finished_at) */;
CREATE TABLE profile_stats_deleted_threads (
      thread_id TEXT PRIMARY KEY,
      project_id TEXT,
      deleted_at TEXT NOT NULL
    );
CREATE TABLE profile_stats_deleted_prompts (
      thread_id TEXT NOT NULL,
      project_id TEXT,
      created_at TEXT NOT NULL
    );
CREATE INDEX idx_profile_stats_deleted_prompts_thread
    ON profile_stats_deleted_prompts(thread_id)
  ;
CREATE TABLE profile_stats_deleted_turns (
      thread_id TEXT NOT NULL,
      provider TEXT,
      model TEXT,
      reasoning TEXT,
      turn_count INTEGER NOT NULL
    , provider_instance_id TEXT);
CREATE INDEX idx_profile_stats_deleted_turns_thread
    ON profile_stats_deleted_turns(thread_id)
  ;
CREATE TABLE profile_stats_deleted_skills (
      thread_id TEXT NOT NULL,
      name TEXT NOT NULL,
      kind TEXT NOT NULL,
      run_count INTEGER NOT NULL
    );
CREATE INDEX idx_profile_stats_deleted_skills_thread
    ON profile_stats_deleted_skills(thread_id)
  ;
CREATE TABLE profile_stats_deleted_tokens (
      thread_id TEXT NOT NULL,
      created_at TEXT NOT NULL,
      provider TEXT,
      tokens INTEGER NOT NULL
    , model TEXT, token_accounting_version INTEGER, provider_instance_id TEXT);
CREATE INDEX idx_profile_stats_deleted_tokens_thread
    ON profile_stats_deleted_tokens(thread_id)
  ;
CREATE TABLE managed_attachment_blobs (
      attachment_id TEXT PRIMARY KEY,
      owner_thread_id TEXT NOT NULL CHECK (length(owner_thread_id) > 0),
      owner_kind TEXT NOT NULL CHECK (length(owner_kind) > 0),
      owner_id TEXT NOT NULL CHECK (length(owner_id) > 0),
      kind TEXT NOT NULL CHECK (kind IN ('image', 'file')),
      original_name TEXT NOT NULL CHECK (length(original_name) BETWEEN 1 AND 255),
      mime_type TEXT NOT NULL CHECK (length(mime_type) BETWEEN 1 AND 100),
      reserved_bytes INTEGER NOT NULL CHECK (reserved_bytes >= 0),
      size_bytes INTEGER CHECK (
        size_bytes IS NULL OR (size_bytes >= 0 AND size_bytes <= reserved_bytes)
      ),
      sha256 TEXT CHECK (
        sha256 IS NULL OR (
          length(sha256) = 64 AND
          sha256 NOT GLOB '*[^0-9a-f]*'
        )
      ),
      relative_path TEXT NOT NULL UNIQUE CHECK (
        length(relative_path) > 0 AND
        relative_path NOT LIKE '/%' AND
        relative_path NOT LIKE '../%' AND
        relative_path NOT LIKE '%/../%' AND
        instr(relative_path, char(0)) = 0
      ),
      state TEXT NOT NULL CHECK (
        state IN ('uploading', 'staged', 'claimed', 'deleting', 'deleted')
      ),
      staging_expires_at TEXT,
      claim_command_id TEXT,
      claim_message_id TEXT,
      claimed_at TEXT,
      delete_reason TEXT,
      delete_requested_at TEXT,
      deleted_at TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      CHECK (
        (state = 'uploading' AND size_bytes IS NULL AND sha256 IS NULL AND
          claim_command_id IS NULL AND claim_message_id IS NULL AND claimed_at IS NULL AND
          delete_reason IS NULL AND delete_requested_at IS NULL AND deleted_at IS NULL) OR
        (state = 'staged' AND size_bytes IS NOT NULL AND sha256 IS NOT NULL AND
          staging_expires_at IS NOT NULL AND claim_command_id IS NULL AND
          claim_message_id IS NULL AND claimed_at IS NULL AND
          delete_reason IS NULL AND delete_requested_at IS NULL AND deleted_at IS NULL) OR
        (state = 'claimed' AND size_bytes IS NOT NULL AND sha256 IS NOT NULL AND
          claim_command_id IS NOT NULL AND claim_message_id IS NOT NULL AND
          claimed_at IS NOT NULL AND delete_reason IS NULL AND
          delete_requested_at IS NULL AND deleted_at IS NULL) OR
        (state = 'deleting' AND delete_reason IS NOT NULL AND
          delete_requested_at IS NOT NULL AND deleted_at IS NULL) OR
        (state = 'deleted' AND delete_reason IS NOT NULL AND
          delete_requested_at IS NOT NULL AND deleted_at IS NOT NULL)
      )
    );
CREATE TABLE managed_attachment_cleanup_jobs (
      attachment_id TEXT PRIMARY KEY,
      reason TEXT NOT NULL CHECK (length(reason) > 0),
      attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
      next_attempt_at TEXT NOT NULL,
      lease_owner TEXT,
      lease_expires_at TEXT,
      last_error TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      FOREIGN KEY (attachment_id)
        REFERENCES managed_attachment_blobs(attachment_id)
        ON DELETE CASCADE,
      CHECK (
        (lease_owner IS NULL AND lease_expires_at IS NULL) OR
        (lease_owner IS NOT NULL AND length(lease_owner) > 0 AND lease_expires_at IS NOT NULL)
      )
    );
CREATE INDEX idx_managed_attachment_blobs_state_expiry
    ON managed_attachment_blobs(state, staging_expires_at, attachment_id)
  ;
CREATE INDEX idx_managed_attachment_blobs_state_reserved
    ON managed_attachment_blobs(state, reserved_bytes)
  ;
CREATE INDEX idx_managed_attachment_blobs_owner_thread
    ON managed_attachment_blobs(owner_thread_id, state, attachment_id)
  ;
CREATE INDEX idx_managed_attachment_blobs_owner_principal
    ON managed_attachment_blobs(owner_kind, owner_id, state, attachment_id)
  ;
CREATE INDEX idx_managed_attachment_blobs_claim
    ON managed_attachment_blobs(claim_command_id, claim_message_id, attachment_id)
  ;
CREATE INDEX idx_managed_attachment_cleanup_jobs_due
    ON managed_attachment_cleanup_jobs(next_attempt_at, lease_expires_at, attachment_id)
  ;
CREATE TRIGGER trg_managed_attachment_blobs_immutable_metadata
    BEFORE UPDATE OF
      owner_thread_id,
      owner_kind,
      owner_id,
      kind,
      original_name,
      mime_type,
      reserved_bytes,
      relative_path
    ON managed_attachment_blobs
    BEGIN
      SELECT RAISE(ABORT, 'managed attachment metadata is immutable');
    END;
CREATE TRIGGER trg_managed_attachment_blobs_immutable_content
    BEFORE UPDATE OF size_bytes, sha256
    ON managed_attachment_blobs
    WHEN
      (OLD.size_bytes IS NOT NULL AND NEW.size_bytes IS NOT OLD.size_bytes) OR
      (OLD.sha256 IS NOT NULL AND NEW.sha256 IS NOT OLD.sha256) OR
      ((OLD.size_bytes IS NULL OR OLD.sha256 IS NULL) AND NEW.state <> 'staged')
    BEGIN
      SELECT RAISE(ABORT, 'managed attachment content identity is immutable');
    END;
CREATE TRIGGER trg_managed_attachment_blobs_state_transition
    BEFORE UPDATE OF state
    ON managed_attachment_blobs
    WHEN NOT (
      NEW.state = OLD.state OR
      (OLD.state = 'uploading' AND NEW.state IN ('staged', 'deleting')) OR
      (OLD.state = 'staged' AND NEW.state IN ('claimed', 'deleting')) OR
      (OLD.state = 'claimed' AND NEW.state = 'deleting') OR
      (OLD.state = 'deleting' AND NEW.state = 'deleted')
    )
    BEGIN
      SELECT RAISE(ABORT, 'invalid managed attachment state transition');
    END;
CREATE TRIGGER orchestration_command_receipts_fingerprint_insert_guard
    BEFORE INSERT ON orchestration_command_receipts
    WHEN (NEW.fingerprint_version IS NULL) <> (NEW.command_fingerprint IS NULL)
    BEGIN
      SELECT RAISE(ABORT, 'command receipt fingerprint fields must both be null or both be set');
    END;
CREATE TRIGGER orchestration_command_receipts_fingerprint_update_guard
    BEFORE UPDATE OF fingerprint_version, command_fingerprint
    ON orchestration_command_receipts
    WHEN (NEW.fingerprint_version IS NULL) <> (NEW.command_fingerprint IS NULL)
    BEGIN
      SELECT RAISE(ABORT, 'command receipt fingerprint fields must both be null or both be set');
    END;
CREATE TABLE IF NOT EXISTS "projection_thread_messages" (
      message_id TEXT NOT NULL,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      role TEXT NOT NULL,
      text TEXT NOT NULL,
      is_streaming INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      attachments_json TEXT,
      source TEXT NOT NULL DEFAULT 'native',
      skills_json TEXT,
      mentions_json TEXT,
      dispatch_mode TEXT,
      dispatch_origin TEXT, sequence INTEGER, text_event_sequence INTEGER NOT NULL DEFAULT 0, text_json TEXT, starts_new_turn INTEGER, async_user_input_json TEXT,
      PRIMARY KEY (thread_id, message_id)
    );
CREATE INDEX idx_projection_thread_messages_message_id
    ON projection_thread_messages(message_id)
  ;
CREATE INDEX idx_projection_thread_messages_thread_created
    ON projection_thread_messages(thread_id, created_at)
  ;
CREATE INDEX idx_projection_thread_messages_thread_created_desc
    ON projection_thread_messages(thread_id, created_at DESC, message_id DESC)
  ;
CREATE INDEX idx_projection_thread_messages_profile_prompt_activity
    ON projection_thread_messages(role, source, created_at)
  ;
CREATE INDEX idx_projection_thread_messages_thread_role_created_desc
    ON projection_thread_messages(thread_id, role, created_at DESC, message_id DESC)
  ;
CREATE TABLE IF NOT EXISTS "projection_pending_interactions" (
      interaction_kind TEXT NOT NULL,
      request_id TEXT NOT NULL,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      lifecycle_generation TEXT,
      status TEXT NOT NULL,
      decision TEXT,
      response_command_id TEXT,
      response_requested_at TEXT,
      created_at TEXT NOT NULL,
      resolved_at TEXT,
      PRIMARY KEY (thread_id, interaction_kind, request_id)
    );
CREATE INDEX idx_projection_pending_interactions_thread_kind_status
    ON projection_pending_interactions(thread_id, interaction_kind, status)
  ;
CREATE INDEX idx_projection_pending_interactions_request_id
    ON projection_pending_interactions(request_id)
  ;
CREATE INDEX idx_projection_thread_messages_thread_sequence
    ON projection_thread_messages(thread_id, sequence, message_id)
  ;
CREATE TABLE orchestration_consumer_state (
      consumer_name TEXT PRIMARY KEY,
      last_acked_sequence INTEGER NOT NULL CHECK (last_acked_sequence >= 0),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
CREATE TABLE orchestration_event_deliveries (
      consumer_name TEXT NOT NULL,
      event_sequence INTEGER NOT NULL,
      thread_id TEXT NOT NULL,
      state TEXT NOT NULL CHECK (
        state IN ('inflight', 'retry', 'succeeded', 'dead', 'uncertain')
      ),
      claim_owner TEXT,
      claimed_at TEXT,
      claim_expires_at TEXT,
      attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
      last_error TEXT,
      completed_at TEXT,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (consumer_name, event_sequence),
      FOREIGN KEY (consumer_name)
        REFERENCES orchestration_consumer_state(consumer_name)
        ON DELETE CASCADE
    );
CREATE INDEX idx_orchestration_event_deliveries_state_sequence
    ON orchestration_event_deliveries(consumer_name, state, event_sequence)
  ;
CREATE INDEX idx_orchestration_event_deliveries_thread_state
    ON orchestration_event_deliveries(consumer_name, thread_id, state, event_sequence)
  ;
CREATE TABLE queued_turn_promotions (
      queued_event_sequence INTEGER PRIMARY KEY,
      thread_id TEXT NOT NULL,
      message_id TEXT NOT NULL,
      dispatch_mode TEXT NOT NULL CHECK (dispatch_mode IN ('queue', 'steer')),
      state TEXT NOT NULL CHECK (state IN ('queued', 'promoting', 'promoted', 'cancelled')),
      claim_owner TEXT,
      claimed_at TEXT,
      claim_expires_at TEXT,
      attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      promoted_at TEXT,
      FOREIGN KEY (queued_event_sequence) REFERENCES orchestration_events(sequence) ON DELETE RESTRICT
    );
CREATE INDEX idx_queued_turn_promotions_thread_state_order
    ON queued_turn_promotions(thread_id, state, dispatch_mode, queued_event_sequence)
  ;
CREATE INDEX idx_queued_turn_promotions_state_expiry
    ON queued_turn_promotions(state, claim_expires_at)
  ;
CREATE UNIQUE INDEX idx_queued_turn_promotions_active_message
    ON queued_turn_promotions(thread_id, message_id)
    WHERE state IN ('queued', 'promoting')
  ;
CREATE TABLE provider_runtime_events (
      sequence INTEGER PRIMARY KEY AUTOINCREMENT,
      event_id TEXT NOT NULL UNIQUE,
      thread_id TEXT NOT NULL,
      turn_id TEXT,
      lifecycle_generation TEXT,
      event_type TEXT NOT NULL,
      event_json TEXT NOT NULL
        CHECK (length(CAST(event_json AS BLOB)) <= 2097152),
      persisted_at TEXT NOT NULL
    );
CREATE INDEX idx_provider_runtime_events_thread_sequence
    ON provider_runtime_events(thread_id, sequence)
  ;
CREATE INDEX idx_provider_runtime_events_turn_sequence
    ON provider_runtime_events(thread_id, turn_id, sequence)
  ;
CREATE TABLE provider_runtime_open_turns (
      thread_id TEXT NOT NULL,
      turn_id TEXT NOT NULL,
      first_sequence INTEGER NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (thread_id, turn_id)
    );
CREATE TABLE provider_runtime_event_consumers (
      consumer_name TEXT PRIMARY KEY,
      last_acked_sequence INTEGER NOT NULL DEFAULT 0
        CHECK (last_acked_sequence >= 0),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
CREATE TABLE provider_delivery_reconciliations (
      reconciliation_id TEXT PRIMARY KEY,
      consumer_name TEXT NOT NULL,
      event_sequence INTEGER NOT NULL,
      thread_id TEXT NOT NULL,
      previous_state TEXT NOT NULL CHECK (previous_state IN ('dead', 'uncertain')),
      outcome TEXT NOT NULL CHECK (outcome IN ('accepted', 'safe_retry', 'abandon')),
      reconciled_by TEXT NOT NULL,
      note TEXT,
      reconciled_at TEXT NOT NULL,
      FOREIGN KEY (consumer_name, event_sequence)
        REFERENCES orchestration_event_deliveries(consumer_name, event_sequence)
        ON DELETE CASCADE
    );
CREATE INDEX idx_provider_delivery_reconciliations_delivery
    ON provider_delivery_reconciliations(consumer_name, event_sequence, reconciled_at DESC)
  ;
CREATE TABLE git_handoff_operations (
      command_id TEXT PRIMARY KEY,
      thread_id TEXT NOT NULL,
      input_json TEXT NOT NULL,
      phase TEXT NOT NULL CHECK (phase IN ('pending', 'git_applied', 'completed', 'uncertain')),
      result_json TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
CREATE INDEX idx_git_handoff_operations_recovery
    ON git_handoff_operations(phase, updated_at, command_id)
  ;
CREATE TABLE project_pull_request_pins (
      project_id TEXT NOT NULL,
      repository_key TEXT NOT NULL,
      pull_request_number INTEGER NOT NULL CHECK (pull_request_number > 0),
      PRIMARY KEY (project_id, repository_key, pull_request_number)
    );
CREATE TRIGGER trg_project_pull_request_pins_limit
    BEFORE INSERT ON project_pull_request_pins
    WHEN
      NOT EXISTS (
        SELECT 1
        FROM project_pull_request_pins
        WHERE project_id = NEW.project_id
          AND repository_key = NEW.repository_key
          AND pull_request_number = NEW.pull_request_number
      )
      AND (
        SELECT COUNT(*)
        FROM project_pull_request_pins
        WHERE project_id = NEW.project_id
      ) >= 20
    BEGIN
      SELECT RAISE(ABORT, 'project pull request pin limit exceeded');
    END;
CREATE TABLE agent_gateway_operations (
      operation_id TEXT PRIMARY KEY,
      caller_thread_id TEXT NOT NULL,
      caller_turn_id TEXT NOT NULL,
      operation_kind TEXT NOT NULL CHECK (operation_kind IN ('create_threads')),
      request_id TEXT NOT NULL CHECK (length(request_id) BETWEEN 1 AND 256),
      fingerprint TEXT NOT NULL,
      requested_count INTEGER NOT NULL CHECK (requested_count BETWEEN 1 AND 20),
      plan_json TEXT NOT NULL,
      status TEXT NOT NULL CHECK (
        status IN ('reserved', 'dispatching', 'completed', 'failed', 'compensating')
      ),
      result_json TEXT,
      error_json TEXT,
      caller_purged_at TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      UNIQUE (caller_thread_id, caller_turn_id, operation_kind)
    );
CREATE INDEX idx_agent_gateway_operations_status
    ON agent_gateway_operations (status, updated_at)
  ;
CREATE INDEX idx_projection_threads_gateway_operation
    ON projection_threads (gateway_operation_id, gateway_operation_index)
  ;
CREATE TABLE operational_diagnostics (
      sequence INTEGER PRIMARY KEY AUTOINCREMENT,
      thread_id TEXT,
      source TEXT NOT NULL CHECK (source IN ('server', 'browser')),
      diagnostic_kind TEXT NOT NULL,
      severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
      code TEXT,
      detail_json TEXT NOT NULL,
      occurred_at TEXT NOT NULL
    );
CREATE INDEX idx_operational_diagnostics_thread_sequence
    ON operational_diagnostics(thread_id, sequence DESC)
  ;
CREATE INDEX idx_operational_diagnostics_occurred_at
    ON operational_diagnostics(occurred_at)
  ;
CREATE TABLE external_mcp_integrations (
      integration_id TEXT PRIMARY KEY,
      name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
      audience TEXT NOT NULL CHECK (audience = 'synara.external-mcp'),
      client_kind TEXT NOT NULL CHECK (
        client_kind IN ('codex', 'claudeCode', 'claudeDesktop', 'other')
      ),
      credential_hash TEXT UNIQUE,
      capabilities_json TEXT NOT NULL,
      created_at TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      last_used_at TEXT,
      paired_at TEXT,
      revoked_at TEXT,
      rate_limit_per_minute INTEGER NOT NULL CHECK (rate_limit_per_minute BETWEEN 1 AND 10000),
      concurrency_limit INTEGER NOT NULL CHECK (concurrency_limit BETWEEN 1 AND 100)
    , project_scope TEXT NOT NULL DEFAULT 'selected'
      CHECK (project_scope IN ('all', 'selected')));
CREATE TABLE external_mcp_integration_projects (
      integration_id TEXT NOT NULL REFERENCES external_mcp_integrations(integration_id)
        ON DELETE CASCADE,
      project_id TEXT NOT NULL,
      PRIMARY KEY (integration_id, project_id)
    );
CREATE TABLE external_mcp_pairing_codes (
      pairing_hash TEXT PRIMARY KEY,
      integration_id TEXT NOT NULL UNIQUE REFERENCES external_mcp_integrations(integration_id)
        ON DELETE CASCADE,
      created_at TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      consumed_at TEXT
    );
CREATE TABLE external_mcp_operations (
      operation_id TEXT PRIMARY KEY,
      integration_id TEXT NOT NULL REFERENCES external_mcp_integrations(integration_id),
      request_id TEXT NOT NULL CHECK (length(request_id) BETWEEN 1 AND 256),
      fingerprint TEXT NOT NULL,
      requested_count INTEGER NOT NULL CHECK (requested_count = 1),
      plan_json TEXT NOT NULL,
      status TEXT NOT NULL CHECK (
        status IN ('reserved', 'dispatching', 'completed', 'failed', 'compensating')
      ),
      result_json TEXT,
      error_json TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      UNIQUE (integration_id, request_id)
    );
CREATE INDEX idx_external_mcp_operations_status
    ON external_mcp_operations (status, updated_at)
  ;
CREATE TABLE external_mcp_tasks (
      integration_id TEXT NOT NULL REFERENCES external_mcp_integrations(integration_id),
      operation_id TEXT NOT NULL UNIQUE REFERENCES external_mcp_operations(operation_id),
      request_id TEXT NOT NULL,
      thread_id TEXT NOT NULL UNIQUE,
      project_id TEXT NOT NULL,
      status TEXT NOT NULL CHECK (status IN ('planned', 'created', 'failed')),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (integration_id, thread_id)
    );
CREATE INDEX idx_external_mcp_tasks_project
    ON external_mcp_tasks (integration_id, project_id, status, updated_at)
  ;
CREATE TABLE external_mcp_audit_log (
      audit_id TEXT PRIMARY KEY,
      integration_id TEXT NOT NULL REFERENCES external_mcp_integrations(integration_id),
      tool TEXT NOT NULL,
      request_id TEXT,
      project_id TEXT,
      runtime_mode TEXT,
      environment TEXT,
      outcome TEXT NOT NULL,
      created_task_ids_json TEXT NOT NULL DEFAULT '[]',
      detail TEXT,
      created_at TEXT NOT NULL
    );
CREATE INDEX idx_external_mcp_audit_rate
    ON external_mcp_audit_log (integration_id, created_at)
  ;
CREATE TABLE external_mcp_rate_windows (
      integration_id TEXT PRIMARY KEY REFERENCES external_mcp_integrations(integration_id)
        ON DELETE CASCADE,
      window_id INTEGER NOT NULL,
      admitted_count INTEGER NOT NULL DEFAULT 0,
      rejected_count INTEGER NOT NULL DEFAULT 0,
      rejection_audit_id TEXT,
      updated_at TEXT NOT NULL
    );
CREATE TABLE projection_spaces (
      space_id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      icon TEXT NOT NULL,
      sort_order INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      deleted_at TEXT
    );
CREATE INDEX idx_projection_spaces_active_order
    ON projection_spaces(deleted_at, sort_order, space_id)
  ;
CREATE INDEX idx_projection_projects_space_id
    ON projection_projects(space_id)
  ;
CREATE TABLE automation_memory (
      automation_id TEXT PRIMARY KEY,
      content TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      FOREIGN KEY (automation_id) REFERENCES automation_definitions(automation_id)
    );
CREATE TABLE automation_settings (
      setting_key TEXT PRIMARY KEY,
      setting_value TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
CREATE TABLE message_text_segments (
      thread_id TEXT NOT NULL,
      message_id TEXT NOT NULL,
      sequence INTEGER NOT NULL,
      started_at TEXT NOT NULL,
      ended_at TEXT NOT NULL,
      text TEXT NOT NULL, text_json TEXT,
      PRIMARY KEY (thread_id, message_id, sequence)
    );
CREATE TABLE message_text_chunks (
      thread_id TEXT NOT NULL,
      message_id TEXT NOT NULL,
      event_sequence INTEGER NOT NULL,
      segment_sequence INTEGER,
      text_json TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (thread_id, message_id, event_sequence),
      FOREIGN KEY (thread_id, message_id)
        REFERENCES projection_thread_messages(thread_id, message_id) ON DELETE CASCADE
    ) WITHOUT ROWID
  ;
CREATE INDEX idx_message_text_chunks_segment ON message_text_chunks(thread_id, message_id, segment_sequence, event_sequence);
CREATE TABLE profile_stats_claude_legacy_usage (
      thread_id TEXT NOT NULL,
      turn_id TEXT NOT NULL,
      tokens INTEGER NOT NULL CHECK(tokens >= 0),
      PRIMARY KEY (thread_id, turn_id)
    ) WITHOUT ROWID
  ;
CREATE TABLE project_import_origins (
      source_key TEXT PRIMARY KEY NOT NULL,
      provider TEXT NOT NULL,
      source_home TEXT NOT NULL,
      external_id TEXT NOT NULL,
      project_id TEXT NOT NULL,
      thread_id TEXT NOT NULL UNIQUE,
      status TEXT NOT NULL CHECK (status IN ('pending', 'completed')),
      created_at TEXT NOT NULL,
      UNIQUE(provider, source_home, external_id)
    );
CREATE TABLE agent_gateway_completions (
    child_thread_id TEXT PRIMARY KEY,
    creator_thread_id TEXT NOT NULL,
    initial_message_id TEXT NOT NULL,
    result_json TEXT,
    delivery_state TEXT NOT NULL DEFAULT 'pending'
      CHECK (delivery_state IN ('pending', 'delivered', 'unavailable')),
    context_event_sequence INTEGER,
    context_consumed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
  );
CREATE INDEX idx_gateway_completions_delivery
    ON agent_gateway_completions(delivery_state, created_at);
CREATE INDEX idx_gateway_completions_context
    ON agent_gateway_completions(creator_thread_id, context_consumed, context_event_sequence);
CREATE TABLE project_agent_configs (
      project_id TEXT PRIMARY KEY,
      coordinator_thread_id TEXT NOT NULL,
      coordinator_name TEXT NOT NULL,
      coordinator_model_selection_json TEXT NOT NULL,
      coordinator_provider_options_json TEXT,
      worker_routing_json TEXT,
      limits_json TEXT NOT NULL,
      capture_enabled INTEGER NOT NULL,
      enabled INTEGER NOT NULL,
      automation_id TEXT,
      revision INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      disabled_at TEXT
    , goal TEXT, icon TEXT, auto_memory_enabled INTEGER NOT NULL DEFAULT 0, library_path TEXT, library_remote_url TEXT, library_push_on_change INTEGER NOT NULL DEFAULT 0, coordinator_icon TEXT, coordinator_color TEXT, paused_at TEXT, archived_at TEXT, paused_automation_ids TEXT);
CREATE UNIQUE INDEX idx_project_agent_configs_coordinator
    ON project_agent_configs (coordinator_thread_id)
  ;
CREATE TABLE project_agent_goals (
      goal_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      objective TEXT NOT NULL,
      authorization_source TEXT NOT NULL,
      scope_version INTEGER NOT NULL,
      acceptance_criteria TEXT,
      limits_json TEXT NOT NULL,
      status TEXT NOT NULL,
      continuation_count INTEGER NOT NULL,
      worker_creation_count INTEGER NOT NULL,
      authorized_at TEXT NOT NULL,
      revision INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      FOREIGN KEY (project_id) REFERENCES project_agent_configs(project_id)
    );
CREATE INDEX idx_project_agent_goals_project
    ON project_agent_goals (project_id, updated_at DESC, goal_id DESC)
  ;
CREATE UNIQUE INDEX idx_project_agent_goals_one_open
    ON project_agent_goals (project_id)
    WHERE status IN ('active', 'paused')
  ;
CREATE TABLE project_agent_tasks (
      task_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      goal_id TEXT NOT NULL,
      title TEXT NOT NULL,
      description TEXT,
      acceptance_criteria TEXT,
      status TEXT NOT NULL,
      assigned_thread_id TEXT,
      repair_count INTEGER NOT NULL,
      archived_at TEXT,
      revision INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL,
      FOREIGN KEY (goal_id) REFERENCES project_agent_goals(goal_id)
    );
CREATE INDEX idx_project_agent_tasks_goal
    ON project_agent_tasks (project_id, goal_id, created_at DESC, task_id DESC)
  ;
CREATE TABLE project_agent_task_dependencies (
      task_id TEXT NOT NULL,
      depends_on_task_id TEXT NOT NULL,
      project_id TEXT NOT NULL,
      PRIMARY KEY (task_id, depends_on_task_id)
    );
CREATE TABLE project_agent_task_attempts (
      attempt_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      task_id TEXT NOT NULL,
      worker_thread_id TEXT NOT NULL,
      gateway_operation_id TEXT,
      request_id TEXT NOT NULL,
      attempt_number INTEGER NOT NULL,
      outcome TEXT NOT NULL,
      error TEXT,
      created_at TEXT NOT NULL,
      finished_at TEXT,
      FOREIGN KEY (task_id) REFERENCES project_agent_tasks(task_id)
    );
CREATE UNIQUE INDEX idx_project_agent_attempts_request
    ON project_agent_task_attempts (request_id)
  ;
CREATE INDEX idx_project_agent_attempts_task
    ON project_agent_task_attempts (task_id, attempt_number DESC)
  ;
CREATE TABLE project_agent_evidence (
      evidence_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      task_id TEXT,
      attempt_id TEXT,
      kind TEXT NOT NULL,
      classification TEXT NOT NULL,
      author_kind TEXT NOT NULL,
      author_thread_id TEXT,
      source_thread_id TEXT,
      source_message_id TEXT,
      source_turn_id TEXT,
      summary TEXT NOT NULL,
      created_at TEXT NOT NULL
    );
CREATE INDEX idx_project_agent_evidence_task
    ON project_agent_evidence (project_id, task_id, created_at DESC)
  ;
CREATE TABLE project_agent_documents (
      revision_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      logical_path TEXT NOT NULL,
      revision INTEGER NOT NULL,
      content TEXT NOT NULL,
      content_hash TEXT NOT NULL,
      author_kind TEXT NOT NULL,
      author_thread_id TEXT,
      sources_json TEXT NOT NULL,
      created_at TEXT NOT NULL
    );
CREATE UNIQUE INDEX idx_project_agent_documents_revision
    ON project_agent_documents (project_id, logical_path, revision)
  ;
CREATE TABLE project_agent_document_heads (
      project_id TEXT NOT NULL,
      logical_path TEXT NOT NULL,
      revision INTEGER NOT NULL,
      content_hash TEXT NOT NULL,
      disk_hash TEXT,
      conflict_pending INTEGER NOT NULL,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (project_id, logical_path)
    );
CREATE TABLE project_agent_activity (
      activity_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      sequence INTEGER NOT NULL,
      kind TEXT NOT NULL,
      actor_kind TEXT NOT NULL,
      actor_thread_id TEXT,
      goal_id TEXT,
      task_id TEXT,
      source_json TEXT,
      summary TEXT NOT NULL,
      created_at TEXT NOT NULL
    );
CREATE UNIQUE INDEX idx_project_agent_activity_sequence
    ON project_agent_activity (project_id, sequence)
  ;
CREATE INDEX idx_project_agent_activity_project
    ON project_agent_activity (project_id, sequence DESC)
  ;
CREATE TABLE project_agent_digests (
      project_id TEXT PRIMARY KEY,
      summary TEXT NOT NULL,
      focus_items_json TEXT NOT NULL,
      coverage_from_sequence INTEGER NOT NULL,
      coverage_to_sequence INTEGER NOT NULL,
      historical_coverage TEXT NOT NULL,
      summarized_thread_count INTEGER NOT NULL,
      pending_thread_count INTEGER NOT NULL,
      generation_state TEXT NOT NULL,
      generated_at TEXT,
      last_good_at TEXT,
      last_error TEXT,
      pinned_focus_json TEXT NOT NULL
    );
CREATE TABLE project_agent_event_inbox (
      inbox_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      source_thread_id TEXT NOT NULL,
      source_event_id TEXT NOT NULL,
      event_type TEXT NOT NULL,
      task_id TEXT,
      eligible_wake INTEGER NOT NULL,
      created_at TEXT NOT NULL
    );
CREATE UNIQUE INDEX idx_project_agent_inbox_source
    ON project_agent_event_inbox (source_event_id)
  ;
CREATE INDEX idx_project_agent_inbox_project
    ON project_agent_event_inbox (project_id, created_at ASC, inbox_id ASC)
  ;
CREATE TABLE project_agent_cursors (
      project_id TEXT PRIMARY KEY,
      processed_through_inbox_id TEXT,
      frozen_from_inbox_id TEXT,
      frozen_to_inbox_id TEXT,
      coordinator_busy INTEGER NOT NULL,
      updated_at TEXT NOT NULL
    , processed_through_created_at TEXT, coordinator_busy_since TEXT);
CREATE TABLE project_agent_thread_index (
      project_id TEXT NOT NULL,
      thread_id TEXT NOT NULL,
      excluded INTEGER NOT NULL,
      archived INTEGER NOT NULL,
      summary_status TEXT NOT NULL,
      last_updated_at TEXT,
      last_summarized_at TEXT,
      PRIMARY KEY (project_id, thread_id)
    );
CREATE INDEX idx_project_agent_thread_index_updated
    ON project_agent_thread_index (project_id, last_updated_at DESC)
  ;
CREATE TABLE project_agent_receipts (
      request_id TEXT PRIMARY KEY,
      project_id TEXT NOT NULL,
      operation TEXT NOT NULL,
      result_json TEXT NOT NULL,
      created_at TEXT NOT NULL
    );
CREATE INDEX idx_project_agent_receipts_project
    ON project_agent_receipts (project_id, created_at DESC)
  ;
CREATE TABLE project_agent_linked_projects (
      project_id TEXT NOT NULL,
      linked_project_id TEXT NOT NULL,
      created_at TEXT NOT NULL,
      PRIMARY KEY (project_id, linked_project_id)
    );
CREATE INDEX idx_project_agent_linked_projects_linked
    ON project_agent_linked_projects (linked_project_id)
  ;
CREATE TABLE project_agent_managed_workers (
      project_id TEXT NOT NULL,
      thread_id TEXT NOT NULL,
      batch_id TEXT NOT NULL,
      request_id TEXT NOT NULL,
      title TEXT NOT NULL,
      task_id TEXT,
      settled_at TEXT,
      settle_outcome TEXT,
      waiting_since TEXT,
      stuck_kind TEXT,
      stuck_since TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL, task_prompt TEXT, recovery_episode TEXT, recovery_step INTEGER NOT NULL DEFAULT 0, nudge_at TEXT, recoveries_used INTEGER NOT NULL DEFAULT 0, needs_you INTEGER NOT NULL DEFAULT 0, needs_you_at TEXT, active_turn_origin TEXT, active_turn_command_id TEXT, result_summary TEXT, result_at TEXT,
      PRIMARY KEY (project_id, thread_id)
    );
CREATE UNIQUE INDEX idx_project_agent_workers_thread
    ON project_agent_managed_workers (thread_id)
  ;
CREATE INDEX idx_project_agent_workers_batch
    ON project_agent_managed_workers (project_id, batch_id)
  ;
CREATE TABLE projection_thread_active_tools (
        thread_id TEXT NOT NULL,
        item_id TEXT NOT NULL,
        turn_id TEXT,
        started_at TEXT NOT NULL,
        started_event_id TEXT NOT NULL,
        PRIMARY KEY (thread_id, item_id)
      );
CREATE INDEX idx_projection_thread_sessions_provider_instance
    ON projection_thread_sessions(provider_instance_id)
  ;
CREATE INDEX idx_provider_session_runtime_provider_instance
    ON provider_session_runtime(provider_instance_id)
  ;
CREATE INDEX idx_projection_turns_pending_message
    ON projection_turns(thread_id, pending_message_id)
  ;
CREATE TABLE todos (
    todo_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'none'
      CHECK (priority IN ('none', 'low', 'medium', 'high', 'urgent')),
    project_id TEXT,
    due_date TEXT,
    thread_id TEXT,
    delegation_base_turn_id TEXT,
    linked_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
  );
CREATE INDEX idx_todos_created_at ON todos(created_at);
CREATE UNIQUE INDEX idx_todos_open_thread
    ON todos(thread_id) WHERE thread_id IS NOT NULL AND completed_at IS NULL;
CREATE TABLE project_import_history (
      thread_id TEXT NOT NULL,
      revision INTEGER NOT NULL,
      state_json TEXT NOT NULL,
      PRIMARY KEY (thread_id, revision)
    );
CREATE TABLE project_agent_work_items (
      sequence INTEGER PRIMARY KEY AUTOINCREMENT,
      work_item_id TEXT NOT NULL UNIQUE,
      project_id TEXT NOT NULL,
      scope_key TEXT NOT NULL,
      task_index INTEGER NOT NULL,
      state TEXT NOT NULL,
      worker_thread_id TEXT,
      slot_held INTEGER NOT NULL DEFAULT 0,
      revision INTEGER NOT NULL,
      record_json TEXT NOT NULL,
      UNIQUE (project_id, scope_key, task_index)
    );
CREATE INDEX idx_project_agent_work_queue
    ON project_agent_work_items (project_id, state, sequence)
  ;
CREATE UNIQUE INDEX idx_project_agent_work_worker
    ON project_agent_work_items (worker_thread_id)
    WHERE worker_thread_id IS NOT NULL
  ;
CREATE TABLE project_agent_work_requests (
      project_id TEXT NOT NULL,
      request_id TEXT NOT NULL,
      scope_key TEXT NOT NULL,
      fingerprint TEXT NOT NULL,
      PRIMARY KEY (project_id, request_id)
    );
CREATE TABLE pull_request_auto_fix (
      thread_id TEXT NOT NULL,
      pull_request_url TEXT NOT NULL,
      requested_pull_request_url TEXT,
      status TEXT NOT NULL,
      pause_reason TEXT,
      attempts INTEGER NOT NULL DEFAULT 0,
      last_handled_head_sha TEXT,
      updated_at TEXT NOT NULL,
      PRIMARY KEY (thread_id, pull_request_url)
    );
CREATE UNIQUE INDEX pull_request_auto_fix_active_owner
    ON pull_request_auto_fix (pull_request_url) WHERE status <> 'paused';
CREATE VIEW external_mcp_active_capacity_claims AS
    SELECT operations.integration_id, operations.operation_id
    FROM external_mcp_operations AS operations
    WHERE operations.status IN ('reserved', 'dispatching', 'compensating')

    UNION

    SELECT tasks.integration_id, tasks.operation_id
    FROM external_mcp_tasks AS tasks
    INNER JOIN external_mcp_operations AS operations
      ON operations.operation_id = tasks.operation_id
    WHERE tasks.status IN ('planned', 'created', 'failed')
      AND COALESCE((
        SELECT CASE
          WHEN turns.state IN ('pending', 'running') THEN turns.state
          WHEN sessions.status = 'error' THEN 'error'
          WHEN sessions.status IN ('interrupted', 'stopped') THEN 'interrupted'
          WHEN EXISTS (
            SELECT 1
            FROM projection_turns AS pending_turns
            WHERE pending_turns.thread_id = threads.thread_id
              AND pending_turns.turn_id IS NULL
              AND pending_turns.state = 'pending'
              AND pending_turns.pending_message_id IS NOT NULL
              AND pending_turns.checkpoint_turn_count IS NULL
              AND (turns.requested_at IS NULL OR pending_turns.requested_at >= turns.requested_at)
          ) THEN 'pending'
          ELSE COALESCE(
            turns.state,
            CASE
              WHEN tasks.status = 'failed' AND operations.status <> 'compensating'
                THEN 'completed'
              ELSE 'pending'
            END
          )
        END
        FROM projection_threads AS threads
        LEFT JOIN projection_thread_sessions AS sessions
          ON sessions.thread_id = threads.thread_id
        LEFT JOIN projection_turns AS turns
          ON turns.thread_id = threads.thread_id
         AND turns.turn_id = COALESCE(
           threads.latest_turn_id,
           (
             SELECT latest.turn_id
             FROM projection_turns AS latest
             WHERE latest.thread_id = threads.thread_id
               AND latest.turn_id IS NOT NULL
             ORDER BY latest.requested_at DESC, latest.turn_id DESC
             LIMIT 1
           )
         )
        WHERE threads.thread_id = tasks.thread_id
        LIMIT 1
      ), CASE
        WHEN tasks.status = 'failed' AND operations.status <> 'compensating' THEN 'completed'
        ELSE 'pending'
      END) IN ('pending', 'running')
/* external_mcp_active_capacity_claims(integration_id,operation_id) */;
CREATE INDEX idx_projection_thread_messages_turn_activity
    ON projection_thread_messages(thread_id, turn_id, updated_at)
  ;
CREATE TABLE account_usage_sync (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      watermark_minute TEXT,
      last_failure_at TEXT
    , account_identity TEXT);
CREATE TABLE remote_device_trust (
    environment_id TEXT NOT NULL,
    root_fingerprint TEXT NOT NULL,
    device_jkt TEXT NOT NULL,
    device_public_key TEXT NOT NULL,
    account_authority TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    label TEXT NOT NULL,
    generation INTEGER NOT NULL,
    approved_at TEXT NOT NULL,
    approved_invite_id TEXT NOT NULL,
    revoked_at TEXT, enrolled_via TEXT NOT NULL DEFAULT 'approval', last_connected_at TEXT,
    PRIMARY KEY (environment_id, root_fingerprint, device_jkt)
  );
CREATE TABLE remote_access_state (
    environment_id TEXT NOT NULL,
    root_fingerprint TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0, allow_connections INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (environment_id, root_fingerprint)
  );
CREATE TABLE remote_host_trust (
    controller_environment_id TEXT NOT NULL,
    account_authority TEXT NOT NULL,
    user_id TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    environment_id TEXT NOT NULL,
    channel TEXT NOT NULL,
    root_certificate TEXT NOT NULL,
    root_fingerprint TEXT NOT NULL,
    host_id TEXT NOT NULL,
    label TEXT NOT NULL,
    paired_at TEXT,
    revoked_at TEXT, desired INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (controller_environment_id, account_authority, user_id, organization_id, environment_id)
  );
CREATE INDEX idx_orch_usage_limit_recovery_source
    ON orchestration_events(stream_id,json_extract(payload_json,'$.usageLimitRecovery.turnId'),sequence)
    WHERE aggregate_kind='thread' AND event_type='thread.meta-updated'
      AND json_type(payload_json,'$.usageLimitRecovery')='object';
CREATE TABLE usage_analytics_deleted_tokens (
      thread_id TEXT NOT NULL,
      project_id TEXT,
      created_at TEXT NOT NULL,
      dispatch_origin TEXT NOT NULL CHECK (dispatch_origin IN ('automation', 'agent')),
      provider TEXT NOT NULL,
      model TEXT NOT NULL,
      tokens INTEGER NOT NULL
    );
CREATE INDEX idx_usage_analytics_deleted_tokens_thread
    ON usage_analytics_deleted_tokens(thread_id)
  ;
CREATE INDEX idx_usage_analytics_deleted_tokens_time
    ON usage_analytics_deleted_tokens(created_at)
  ;
