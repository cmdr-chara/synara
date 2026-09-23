# Native workflow slash commands

Type `/` or `/synara/` in the composer to see the native command menu. Select a
row or type the exact command and press Send. Native commands are accepted only
at the start of the entire trimmed draft, without arguments. Unknown names and
extra text remain in the draft and are never sent to a provider.

The qualified namespace avoids collisions: ACP-advertised command names do not
allow an embedded slash. `/plan`, `/debug` and other provider command names remain
provider-owned. Native commands do not change their interpretation.

| Command | Native action |
| --- | --- |
| `/synara/plan` | Select the connected ACP session's advertised Plan mode through the existing control owner. Unsupported/direct-model sessions retain the command and report the limitation. |
| `/synara/debug` | Open the evidence-first Debug workflow. |
| `/synara/goal` | Open goal review without setting, changing or arming a goal. |
| `/synara/recap` | Open recap review without automatically generating a response. |
| `/synara/fork` | Create a new unsent same-checkout branch through the last saved assistant turn. No hidden state, tools or permissions are copied. |
| `/synara/subagents` | Open Workflows settings without starting agents. |
| `/synara/automation` | Open automation review without arming the scheduler. |
| `/synara/computer-use` | Open Computer Use settings without selecting, observing or controlling a window. |
| `/synara/status` | Open reported Usage details. Missing provider telemetry remains unknown. |
| `/synara/export` | Open the system save dialog for the existing Markdown text export. |
| `/synara/export-zip` | Save a finished conversation as a compressed ZIP containing `thread.json` and `transcript.md`. Both entries share one durable snapshot. |

The normal Send entry point applies task, loading, busy, connection, IME, pending
control and draft ownership guards. The Stop button remains Stop. Menu callbacks
recheck the exact task and text before replacing a partial command. Successful
commands consume only their exact command line, not attachments or other drafts.
Native command drafts survive restart as unsent text and never auto-execute.

This is intentionally not full upstream command parity: argument forms, the
unqualified shared namespace, provider-native forks and worktree selection
remain open. ZIP export includes text and recorded image metadata, not image bytes,
unsent drafts, credentials, tool payloads or workspace files. `/synara/computer-use` opens review, whereas upstream's
`/computer-use <task>` has per-request execution semantics.

See [verification](../verification/parity-continuation-2026-09-23.md).
