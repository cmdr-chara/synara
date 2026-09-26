# Real-account provider acceptance

Roadmap owners: **A04** (fresh-install onboarding) and **A09** (ACP/direct-model
interoperability). Both remain **OPEN** until the real workflows below have
candidate-bound evidence. Fixture success and missing credentials never close a
live-account gate.

## Executable direct-provider matrix

`crates/synara-model/tests/provider_live.rs` exercises Synara's actual
`HttpModelProvider` with reviewed account credentials. Ordinary `cargo test`
ignores the live test. There is no automatic workflow and no account lookup.

The operator explicitly supplies **3-12** profiles covering all three implemented
protocols: OpenAI-compatible chat, Anthropic Messages and Google GenerateContent.
Each cell checks:

1. A request cancelled before submission returns `Cancelled`, emits no events and
   does not read its credential.
2. One authenticated text stream completes with the requested synthetic marker.
3. A second authenticated request carries the first user/assistant exchange as
   history and answers a follow-up about that marker.

Each real request has a 90-second deadline and a reviewed output budget of
32-2048 tokens. A cell sends at most **two paid requests**, never retries and never
offers or executes tools. A failed first request prevents its follow-up. The
matrix proceeds to other reviewed cells so each has a result. Reported token
usage is recorded as present or absent without inventing provider telemetry.

Credentials are read only from the explicitly named `SYNARA_PROVIDER_KEY_*`
environment variables. All bindings are checked before any network request.
The in-memory credential owner binds each key to exactly one canonical profile,
protocol and endpoint. It never writes to the OS key store, workspace database,
configuration or report. Environment variables remain under the operator's
process ownership, so unset them after the run.

The report includes the exact commit, platform, profile IDs, protocol, a hash of
each reviewed route and allowlisted results. It excludes credential values,
endpoint URLs, model metadata, prompts, transcripts, reasoning and provider error
bodies. The output must be a new file and is created with mode `0600` on Unix.
Partial reports are flushed after each provider. A reported pass also requires
the same commit and unchanged tracked checkout before and after the matrix.

### Configure and run

Prepare a JSON file **outside the checkout**. Replace every `REVIEWED_MODEL_ID`
with the model you intend to use and `FULL_CANDIDATE_COMMIT` with the checkout's
full 40-character commit. Model names are intentionally not guessed from a
changing provider catalog. Endpoints and account/model access require operator
review before opting in.

```json
{
  "candidate_commit": "FULL_CANDIDATE_COMMIT",
  "providers": [
    {
      "profile": {
        "id": "openai-account",
        "name": "Reviewed OpenAI account",
        "protocol": "open_ai_chat",
        "endpoint": "https://api.openai.com/v1",
        "requires_key": true,
        "models": [{"id": "REVIEWED_MODEL_ID", "name": "Reviewed model"}]
      },
      "model_id": "REVIEWED_MODEL_ID",
      "key_env": "SYNARA_PROVIDER_KEY_OPENAI",
      "max_output_tokens": 256
    },
    {
      "profile": {
        "id": "anthropic-account",
        "name": "Reviewed Anthropic account",
        "protocol": "anthropic_messages",
        "endpoint": "https://api.anthropic.com/v1",
        "requires_key": true,
        "models": [{"id": "REVIEWED_MODEL_ID", "name": "Reviewed model"}]
      },
      "model_id": "REVIEWED_MODEL_ID",
      "key_env": "SYNARA_PROVIDER_KEY_ANTHROPIC",
      "max_output_tokens": 256
    },
    {
      "profile": {
        "id": "google-account",
        "name": "Reviewed Google account",
        "protocol": "google_generate_content",
        "endpoint": "https://generativelanguage.googleapis.com/v1beta",
        "requires_key": true,
        "models": [{"id": "REVIEWED_MODEL_ID", "name": "Reviewed model"}]
      },
      "model_id": "REVIEWED_MODEL_ID",
      "key_env": "SYNARA_PROVIDER_KEY_GOOGLE",
      "max_output_tokens": 256
    }
  ]
}
```

Supply the three credential variables through the operator's existing secret
environment mechanism. Never paste keys into command history, configuration,
issues or evidence. From the reviewed clean checkout:

```sh
export SYNARA_PROVIDER_CONFIG=/absolute/private/provider-matrix.json
export SYNARA_PROVIDER_REPORT=/absolute/private/new-provider-report.json
export SYNARA_PROVIDER_ACCEPTANCE=reviewed-paid-accounts
cargo +1.98.1 test --locked -p synara-model --test provider_live \
  reviewed_live_direct_provider_matrix -- --ignored --exact
unset SYNARA_PROVIDER_ACCEPTANCE SYNARA_PROVIDER_KEY_OPENAI \
  SYNARA_PROVIDER_KEY_ANTHROPIC SYNARA_PROVIDER_KEY_GOOGLE
```

Missing opt-in, an incomplete protocol matrix, missing credentials, insecure
endpoints, an existing report or a mismatched/dirty tracked candidate fail
before paid requests. Authentication rejection, quota/rate limits, incomplete
streams and deadlines fail the relevant cell, with no automatic retry. The
ordinary credential-binding/configuration tests can run without accounts:

```sh
cargo +1.98.1 test --locked -p synara-model --test provider_live
```

## Evidence still required

| Acceptance cell | Existing executable evidence | Real-account evidence still required |
| --- | --- | --- |
| Fresh native setup and sign-in (A04) | `native_setup_worktree_controls_smoke.py` proves fixture setup remains inert until Connect/login | Fresh native app data, actual advertised provider authentication, project creation, first prompt, restart/replay without unintended resubmission |
| ACP installation/initialization (A09) | `vendor_probe.rs` installs checksum-pinned OpenCode/Gemini releases without credentials | Actual authenticated sessions and prompts with reviewed versions and accounts |
| Direct text/history (A09) | `provider_live.rs` is an opt-in real transport harness | Passing candidate report for all three protocol families |
| ACP model/configuration and recovery (A09) | Process/lifecycle fixtures | Supported model selection, cancellation, reconnect/load/resume and honest unsupported results on real agents |
| ACP/direct handoff (A09) | Native handoff and durable route transaction fixtures | Explicit ACP-to-direct and direct-to-ACP continuation, preserved workspace/task ownership, unsent review, restart and later authorized send |
| ACP provider-native fork (A09) | Capability-gated fixture lifecycle | Real advertised fork plus load/resume support, or recorded unsupported outcome and retained-context fallback |
| Account failure paths (A04/A09) | Protocol/credential denial fixtures | Cancelled/rejected login, revoked/expired account access, rate limit and recovery without duplicate prompts |

The direct harness does not prove native onboarding, the native credential
picker/store, ACP compatibility, active remote cancellation, tool execution,
multimodal inputs, all models, or cross-provider handoff. Pre-cancellation proves
that an unsent request stays local. It does not claim that an already submitted
request stopped processing at the provider. Full A04/A09 closure needs the
remaining native and ACP journeys on the same accepted candidate.
