# Parity batch 10: direct-model presets and context visibility

M25 partial: the direct-model route review exposes Fast, Balanced and Thinking
controls when the selected OpenAI-compatible model advertises a corresponding
reasoning level. Fast chooses low, minimal or none; Balanced chooses medium;
Thinking chooses high, xhigh or max. The controls only edit the existing route
draft. A route confirmation saves the choice and Send initiates inference.

When discovery supplies a context window, the review also displays that window
and the requested output budget. History, attachments and the prompt compete for
the remaining context; the UI does not claim a precise token count or compact
history automatically. M25 remains open for compaction and broader controls.

Validation: source review and whitespace check. Native compile/UI acceptance pending.

M04: a web task with an existing native-reviewed direct-model binding can run
through the same interruptible Controller path as the native composer. GET run
status reports only provider/model IDs and an opaque route stamp, not credentials
or the complete provider configuration. Explicit Run posts that stamp with the
expected draft; the server rejects stale routes and the worker checks again
before submission. Direct-model confirmation explains history transfer, absent
agent tools and possible provider charges. Stop and one-hour timeout retain the
existing cancellation path. Web route selection and provider credential setup
remain native settings operations.
