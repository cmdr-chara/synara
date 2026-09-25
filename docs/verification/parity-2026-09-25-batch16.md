# Parity batch 16: onboarding and voice minor-feature completion

This batch closes two smaller roadmap items whose remaining live-provider and
platform proof is already owned by separate acceptance gates.

## S01 complete: provider-specific onboarding setup/login UX

Native onboarding already creates an explicit unsent setup task and exposes
Connect, agent-advertised ACP authentication methods and connection questions.
The provider guidance added in batch 15 covers Codex, Claude Code, OpenCode and
Gemini CLI. A copyable command is shown only when the configured executable is
the known provider binary; wrapper/custom commands receive guidance without an
invented command.

That completes the product-side S01 surface. It does not claim a real account is
signed in, healthy, entitled or within quota. Fresh-install proof with real
provider accounts remains A04 and the broader D1 gate remains OPEN.

## S02 complete: voice interaction controls

The composer voice loop now includes explicit start, stop-to-transcribe and
cancel controls, navigation/stale-result fencing, the existing two-minute
capture bound, editable unsent-draft insertion, elapsed recording feedback and
a five-level live input meter.

That completes S02 as a product feature. Live microphone + ChatGPT
transcription, macOS permission/package behavior and Windows package behavior
remain A01-A03, and M1 remains OPEN until those acceptance journeys pass.

## Verification

The implementation is carried by
`0848e7de66df9964e84566689b5d9a48d7655390`, with follow-up formatting and
regression corrections integrated before this roadmap close. Batch 15 records
the integrated native regression result and its unrelated pre-existing
`synara-server` response-shape failure.

This commit changes roadmap/evidence only. It does not claim additional
provider, microphone or platform execution beyond the existing batch-15
evidence.
