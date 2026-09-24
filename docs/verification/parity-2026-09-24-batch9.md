# Parity batch 9: web question drafts

M03 is implemented in the local web workspace. Each active question saves its
unfinished field values in the browser profile under the task path and one-shot
request receipt. Returning to the task or restarting the browser restores fields
only when the same request and field schema are still pending. Stored drafts expire
after 24 hours; submitted, declined, cancelled and expired requests clear them.
Users can clear saved answers directly. Storage failures leave the live form usable
and show a warning. The bearer token is never stored with answers.

This covers browser navigation/restart while the local server and its agent request
remain active. A server restart cancels pending one-shot requests, so their answers
cannot be restored as a live request. The browser stores answers locally until they
expire or are cleared; use a trusted browser profile for sensitive answers.

Validation: implementation and diff reviewed. Runtime/browser acceptance is pending.

M02 partial: the web permission panel now includes the matching durable tool title,
kind and at most two bounded recorded diffs. Missing tool context is stated rather
than guessed. The agent still supplies the one-time permission choices. Provider
command arguments and live proposed diffs are not guaranteed in the existing tool
record, so this does not close M02.

M21 partial: the editor comparison panel can copy the displayed reference-vs-buffer
review with added, removed, unchanged and informational line markers. Copy is
disabled while loading, on errors and if the display was shortened, preventing an
incomplete review from appearing complete. This is review text, not an applicable
patch; hunk editing and staging remain open.
