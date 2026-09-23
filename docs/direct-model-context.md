# Reviewed direct-model context

This batch extends the native GPUI direct-chat route. It does not add a second
agent or task owner, execute tools, grant desktop control, or launch ACP agents.

## Attachments and persistence

Select or paste the existing owned PNG/JPEG or plain-text attachments in the
composer. Images require explicitly reviewed `images: supported` metadata for the
selected model. Unknown or unsupported capabilities fail before recording a
prompt. The same bounded intake worker validates file bytes and checks the
attachment revision. Stop remains owned by the existing task cancellation token.

The selected image bytes, MIME type and capture provenance are persisted using
native image events. Subsequent direct turns include retained user images by
exact message identity. Plain-text attachment content is visibly appended to its
user message and therefore survives restart and subsequent context projection.
No URLs, original file paths or files on disk are implicitly fetched later.

The initial local echo matches the existing attachment receipt exactly. A local
receipt is not proof of provider delivery. Recent attachments still require an
explicit reuse action. Unsupported or stale input leaves the pending selection
unchanged. Audio, PDFs, generated images and direct tool execution are not added.
Assistant-returned image history is rejected with a smaller-window review hint,
not silently dropped or relabelled as user input.

## History and generation controls

The model review has native presets for all history, the last 10 prior user turns,
or the current message only. `history_turns` also accepts custom integers 0-256
in the existing JSON review. Older bindings default to `null` (all history).
A turn includes its user message, images and following visible assistant replies.
The current prompt is always added separately. Hidden reasoning, permissions,
sessions and tool state are not copied. The complete local transcript is retained.

Output-token presets respect the reviewed model maximum. Reasoning presets are
shown only for effort values reviewed on the supported OpenAI-compatible transport.
These controls edit the review draft only. Confirmation is still revision- and
transcript-sequence checked. No preset, confirmation, reload or restart sends a
model request. Explicit Send is required.

Requests are preflighted against both normalized and encoded 4 MiB input limits.
Provider configuration remains limited to 1 MiB. Attachment intake remains eight
items / 2 MiB combined. This is a byte budget, not a claimed token count or a
provider context-window guarantee. Oversized requests fail without silent trimming.

## Discovery

Anthropic-compatible discovery uses `limit=1000` and the documented `after_id`
cursor at the exact reviewed endpoint. It reads at most eight pages and 4096
identities. Malformed pagination, repeated identities/cursors, errors and an
unfinished last page reject the entire discovery rather than presenting a partial
collection as complete. Cancellation and the existing discovery deadline cover
all pages. Provider-supplied continuation URLs are never followed.

Reported image-input support and positive input/output token limits are proposed
for newly discovered Anthropic models. Missing metadata remains unknown. Existing
reviewed capabilities are not silently replaced. Tool, structured-output and
reasoning support are not inferred from model names or unrelated capability flags.
Discovery is still a review-before-save operation, not automatic enrollment.
OpenAI-compatible responses advertising additional pages without a reviewed
pagination contract fail explicitly. Existing Google token pagination is unchanged.

Protocol reference: https://platform.claude.com/docs/en/api/models/list

## Evidence and limitations

Added unit and loopback integration tests cover local echo, persistent image
provenance, visible text-file replay, exact image ownership, history windows,
legacy binding defaults, invalid options, unsupported inputs, stale revisions,
cancellation, wire limits, padding validation, multi-page discovery and denial
cases. Native GPUI regression checks remain required for the final candidate.

Real paid-provider authentication, macOS/Windows acceptance, Computer Use, native
subagents/workflows, Agent Gateway and external MCP clients remain outside this
batch. Tests use only isolated native fixtures and owned loopback services.
