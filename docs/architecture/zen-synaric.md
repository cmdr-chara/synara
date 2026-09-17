# Shared GUI architecture: Synaric and Zen

Status: approved product direction and implementation specification. Neither new
layout is implemented by this document. Keep existing roadmap IDs A-Q unchanged
while the other implementation sessions refer to them. The proposed R/S/T lanes
are in [the GUI roadmap supplement](../roadmap/zen-synaric.md).

## Product contract

Synara is one local-first Rust/GPUI GUI application with two layouts, not two
backends or independent databases. Synaric opens by default. Zen is an extremely
minimal alternative. The layout preference is global, not per task or workspace.
No Synara account is required. Platform priority is macOS Apple Silicon, Windows,
then Linux. Retain Linux verification without confusing it with product priority.

Synaric keeps conversation central while exposing useful workspace surfaces.
Zen keeps sessions and conversation visible, revealing tools contextually.
The two are not a feature-unlocked versus feature-restricted product tier.
Neither layout should become a terminal-first application or an overcrowded IDE.

## Single source of application state

The existing domain/service crates continue owning tasks, events, durable storage,
agent connections and execution hosts. Shared GUI components project that state.
Switching layouts must not create another Controller, ACP process, session,
subscription, terminal or browser profile.

Suggested presentation organization after parallel branch integration:

```text
shared presentation
  conversation + composer + tool cards + permissions + questions
  model/config picker + connection status + theme tokens
layouts
  synaric: conversation-first workspace with contextual side panels
  zen: minimal session navigation and contextual overlays
application services
  one task/session/workspace identity and event stream
```

This organization is a direction, not permission to rename or restructure the
shared crates while BCD/AJM/FGH still own active edits.

## Layout switching

One validated setting represents `synaric` or `zen`, defaulting to `synaric` when
missing. Persist a global preference through the settings owner. Unknown settings
must be handled explicitly without losing the rest of the user's configuration.

On a switch, preserve active project/task, transcript logical scroll anchor,
composer draft/selection, active model and traits, connection/session, active
prompt, tool state, pending questions and permissions, open documents and dirty
buffers, terminal ownership and browser tabs. A changed width requires anchor
restoration by stable row identity, not blindly reusing an old pixel offset.

IME preedit must survive or the layout change must defer until composition ends.
The switch must never send a draft, cancel a prompt, approve a permission or close
a dirty file. Restore focus by semantic control identity. When the old control
is hidden, choose a predictable visible target without erasing its data.

Per-layout panel visibility and sizes are presentation preferences, not duplicated
domain state. If multiple application windows are supported, apply the global
choice consistently without sharing a window-specific focus handle.

## Synaric default layout

Use restrained navigation for workspaces and tasks, the shared transcript and
composer, and a contextual side surface for files/diff/browser or other tools.
Keep terminal, Git and inspector available without permanently exposing all of
them. Empty states should make the next action obvious: open a workspace, create
a task, connect an agent, or authenticate. Errors are actionable, not raw dumps.

Avoid a second toolbar for model effort when the unified picker is present.
Advanced tools should not steal transcript scroll, input focus or permission state.

## Zen layout

Use a compact session sidebar, a readable conversation column and the shared
composer. Provide keyboard and discoverable pointer access to the same tools.
Show a subtle activity/permission indicator when relevant. Collapse empty chrome.
Contextual panels may be narrower, but hiding them must not stop their services.
Do not hide unresolved permission requests behind the promise of minimalism.

## Shared model picker: PR #1252 behavioral reference

Reference is the PR description and screenshots only:
https://github.com/Emanuele-web04/synara/pull/1252

Reviewed PR head: `6c669289658eea269bb28bc37157cddf24ef8ed4`.
Do not inspect, copy, translate or port the legacy React implementation. Author the
native component independently. The reference screenshots use fixtures and are
not proof of live-provider compatibility.

The same component and preset store serve both layouts:

- Starred first, then connected-provider tabs in user order. Respect hidden
  providers and show an actionable reason for unavailable choices. The add control
  opens provider/agent settings rather than inventing a catalog entry.
- Search model rows, star controls, and visible-row Command/Ctrl+1 through 9.
  Enter chooses the top visible search result. Tab/Shift+Tab navigate provider
  tabs. Global task shortcuts yield only while the picker owns focus.
- Hover or keyboard navigation opens the effort choices for that model. Selecting
  an effort chooses model plus effort as one user intent. A direct row selection
  retains the current effort only when that value remains valid for the new model.
- Footer trait controls include effort, speed, thinking, context or other exposed
  options. Unsupported options are absent or clearly unavailable. Changing a
  trait keeps the picker open. The optional effort slider preserves keyboard
  stepping, reset and fast-toggle behavior where supported.
- A starred preset saves the model and supported trait combination. The same
  model can have multiple presets. Restoring a preset revalidates it against the
  current agent/session catalog. Missing or stale choices are not silently mapped
  to an unrelated model. Model cycling prefers eligible starred presets.
- Session/provider locking reflects actual backend constraints. Show the count
  of hidden cross-provider presets when locked. Never switch the underlying agent
  or create a replacement session as a side effect of clicking a model row.

## Agent, provider and model are different identities

An agent is an ACP executable/connection. A provider is a model service available
to that agent. A model is an opaque identifier in the agent's catalog. Preserve
these identities and the execution-host/session context in selection state.
Do not infer provider identity by splitting an arbitrary model ID or display name.

OpenCode documents a broad built-in catalog. Users should not have to hand-enter
known providers and models. Provider support does not imply an account is connected
or that every model is available in a particular project. Nor does it guarantee
that the full catalog is exposed through the currently negotiated ACP API.

Consume available agent configuration/catalog data. When provider-group metadata
is unavailable, show the supplied models under an honest agent-level grouping,
not fabricated provider tabs. A separately documented compatibility adapter may
fill a proven discovery gap. It must remain narrow and must not become a duplicate
provider runtime or a hardcoded list of supposedly available models.

## Configuration application is not an assumed atomic protocol operation

ACP session config changes target one option and return updated configuration.
A model change can alter the allowed reasoning/trait values. The picker must
therefore separate a desired preset from acknowledged agent state:

1. Freeze one selection intent with the current connection/session/catalog identity.
2. Serialize configuration changes for that session. Apply model first if needed.
3. Re-read the returned configuration and validate the remaining requested traits.
4. Apply only still-supported values, correlating acknowledgments to that intent.
5. Prevent a new prompt from being sent with a misleading half-applied preset.
6. On partial failure, show actual acknowledged state and an explicit recovery
   action. Do not claim rollback or remote atomicity when neither was proven.

An agent-driven configuration notification updates the same shared state. It
must not trigger an infinite client reapply loop or cross into another task.
Boolean and unknown option types follow negotiated support. Use config options
when available and a documented legacy fallback otherwise, not both simultaneously.

## Visual and accessibility contract

Define shared spacing, typography, contrast and focus tokens. Support light/dark
appearance and reduced motion. Use native text/IME and a complete keyboard route,
not hover as the only way to reveal effort controls. Accessible labels must expose
selected model, current effort, pending state and unavailable reasons.

Manual acceptance is still required for VoiceOver on macOS, the supported Windows
screen reader, Linux accessibility, IME, bidirectional text and display scaling.
A component name or a unit test does not establish accessibility.

## Privacy and updater requirements

Diagnostics default off and use a strict allowlisted payload. Do not send raw
logs, exception strings, project paths, URLs, prompts, transcripts, files, screen
captures, email, credentials, device IDs or arbitrary provider payloads. A local
support export is a separate explicit action, not automatic diagnostics.

The built-in updater needs authenticated/signed metadata and artifacts, platform
matching, explicit consent, interruption recovery, rollback and data compatibility.
Signing identities, update endpoint and release authorization remain owner gates.
Nothing in this specification authorizes uploading user data or issuing a release.

## Primary protocol/product references

- ACP configuration behavior:
  https://agentclientprotocol.com/protocol/v1/session-config-options
- OpenCode provider setup and catalog behavior:
  https://opencode.ai/docs/providers
- OpenCode project-specific model availability:
  https://opencode.ai/v2/docs/models

The layout, state-ownership and recovery requirements above are Synara product
and engineering decisions. Referenced products are behavioral inspiration only.
