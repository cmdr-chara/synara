# Native subagents, Agent Gateway, incoming MCP and Computer Use

These workflows use the native GPUI shell, existing Controller task/session
ownership, SQLite writer and bounded native process runner. They are not a new
provider-specific agent backend. Opening a page or restoring the application does
not launch agents, enroll clients, capture pixels or grant input authority.

## Native subagents and workflows

Open **Subagents & workflows** from the conversation strip or Settings search.
Create a reviewed graph of 1-8 child steps with explicit agent IDs, instructions
and zero-based dependencies on earlier steps. Concurrency is 1-4. Creation saves
ordinary child tasks and unsent drafts atomically. Run is a separate action.

Each child has its own thread, session, permissions, usage and transcript. Open a
child to answer its existing permission requests. Children share the parent's
working folder. This is not automatic worktree isolation: parallel steps can
conflict on files, so only run a graph whose instructions and concurrency have
been reviewed. No parent conversation, hidden reasoning, credentials or approval
history is copied. Only explicitly declared preceding results enter dependency
context, labeled as untrusted output and bounded to 32 KiB per result.

Pause and Stop cancel active owned children and prevent pending steps from being
launched. Existing side effects are retained. Interrupted steps require an
explicit retry decision, with at most three attempts. Recovery after interruption
is inert and does not retry any prompt. Edits to child drafts, profiles, routes or
transcripts invalidate dispatch rather than silently overwriting human work.
Graph identity and revision fence every reviewed mutation. Archiving a parent or
linked child is refused while the workflow owns a run. Stop or recover it first.
Detach removes graph
links only and keeps child tasks, drafts, transcripts and files. Linked children
cannot be permanently deleted until the idle graph is explicitly detached.

## Agent Gateway

Connect the selected local ACP agent, then explicitly enable Agent Gateway. The
connection must advertise HTTP MCP support. Enabling retires this task's session
and starts no prompt. The next explicit Send uses a fresh session containing the
scoped gateway endpoint. The prior native transcript remains local. A changed
agent/profile, Stop, restart or revocation invalidates the transient capability.

The gateway exposes only the owning task's workflow metadata and a request queue.
An agent may propose child creation, running, pausing/stopping, editing a pending
step, preparing a retry, sharing an observed screenshot or delivering reviewed
window input. None of those proposals approves itself. The native user reviews
the complete frozen operation and approves it once or denies it. Approved Run
receipts return bounded child reports and usage to that requesting client, marked
as untrusted output. Status reads alone do not disclose those reports. Children cannot
recursively enroll gateways or own another delegation graph.

## Incoming external MCP clients

This is **clients connecting to Synara**, not the existing feature which passes
third-party MCP servers to an ACP agent. Explicit external enrollment creates an
independent ephemeral loopback endpoint and a 15-minute bearer lease. Copy private
client configuration only through the native clipboard button. No token, endpoint
lease or authority is persisted. Reconnect after restart or expiry explicitly.

The server implements stateless MCP Streamable HTTP with JSON responses. Supported
protocol negotiation is 2025-03-26, 2025-06-18 and 2025-11-25. It has four tools:
`synara_status`, `synara_request`, `synara_result`, and `synara_cancel`. There is no
approval RPC, arbitrary task selector, filesystem reader or process launcher.
Use a stable nonce when proposing an operation. Repeating the same nonce and
payload returns its receipt, never a second execution. Changed payloads with an
old nonce fail. Results and cancellation are scoped to the original client.

Receipts expire if not approved within two minutes. At most eight clients, sixteen
receipts per client, 128 total receipts and bounded result storage are retained.
Revoke a client to cancel its queued/active operations and erase its transient
results. TCP disconnection alone does not imply cancellation. Cancel explicitly
or revoke the lease. Failed or cancelled effects may already exist, and neither
the client nor Synara automatically retries them.

Only literal loopback endpoints are created. Exact HTTP authority and bearer
authentication, duplicate-header rejection, no browser Origin, strict framing,
bounded concurrent peers and absolute read/write deadlines limit the transport.
Remote endpoints, SSH tunnels, anonymous access, OAuth enrollment, notifications
via long-lived SSE and automatic client configuration are outside this slice.

## Computer Use

Open **Computer use**. Explicit discovery checks installed Linux/X11 helpers and
lists application windows. Select one target, observe its preview, and separately
review each input. AppSnap capture consent is never reused as input consent.
The native UI supports local operation as well as approved gateway/client requests.

Computer Use requires `/usr/bin/xwininfo`, `/usr/bin/xprop`, `/usr/bin/import`
(ImageMagick) and `/usr/bin/xdotool`, installed by the user through the operating
system. Nothing is downloaded by the feature. Synara's own approval window, root
and desktop/dock windows are excluded. The selected target lease lasts fifteen
minutes. Each observed frame lasts sixty seconds and permits one reviewed action.
An incoming observation request is pinned to the selected target's identity epoch.

A fresh capture and exact window identity/pixel comparison precede input. Changing
pixels or identity refuses the action and consumes the old frame, requiring
another observation and review. This deliberately rejects highly dynamic screens
rather than acting on stale coordinates. Take over / revoke, task switching and
shutdown cancel pending input and observation. No target or frame lease restores
after restart. Audit notices contain operation identities, not typed text or keys.

Input is explicitly addressed to the selected X11 window. Printable ASCII typing
is bounded to 512 bytes. Supported keys are Enter, Tab, Escape, Backspace, Delete,
arrow keys, Home, End, Page Up/Down and Space. Click and bounded scroll use reviewed
window-relative coordinates and move the shared pointer. There is no global
shortcut, arbitrary command, ambient-focus fallback, held-key macro or automatic
retry. Applications may reject XSendEvent input. A delivery attempt is not proof
of application-level success: explicitly observe the target again.

This is bounded application-window control, not complete Electron Computer Use
parity. Full desktop control, Wayland, macOS, Windows, richer Unicode/IME input,
modifier chords, drag operations and production application acceptance remain
open. X11 is not an OS security sandbox against other malicious X11 clients.

## Verification

`autonomy_integration.rs` exercises real owned ACP processes and authenticated
loopback MCP. `native_autonomy_smoke.py` exercises GPUI controls, frozen approval,
actual X11 target input, unrelated-window exclusion, stale target rejection and
inert restart with independently created fixtures. Unit tests cover graph bounds,
atomic storage/rollback, stale writes, nonce ownership, expiry and protocol bounds.
The accompanying verification receipt records actual execution outcomes. A source
implementation or fixture success alone does not establish paid-provider,
production-client or cross-platform release acceptance.

References:
- https://modelcontextprotocol.io/specification/2025-11-25/basic/transports
- https://modelcontextprotocol.io/specification/2025-11-25/server/tools
- https://github.com/jordansissel/xdotool/blob/main/xdotool.pod
