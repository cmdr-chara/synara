# Live acceptance lanes

The remaining environment-bound acceptance gates are intentionally separate from
hosted compile/unit evidence. A gate stays open until a run against the required
real environment succeeds for an exact reviewed commit.

## A01 - live microphone + ChatGPT transcription

Workflow: `.github/workflows/live-voice-acceptance.yml`.

Required runner label: `synara-live-voice`.

The runner must have a working default microphone, operating-system microphone
permission, `codex` on `PATH`, and Codex already authenticated with ChatGPT.
The ignored acceptance test records four seconds, requires non-zero live input,
resolves the ChatGPT session through the real Codex app-server, uploads only to
the official ChatGPT transcription origin, and requires a non-empty transcript.
The transcript text is not printed or uploaded.

A hosted runner or a synthetic WAV does not close A01.

## A04 - fresh-install onboarding with a real provider account

Workflow: `.github/workflows/live-onboarding-acceptance.yml`.

Required runner label: `synara-live-provider`.

Provider credentials remain in the provider's own pre-authenticated runner HOME.
The workflow input contains only a bounded provider profile
(`id/name/command/args/inherit_env`), never credentials.

The journey starts with a fresh Synara data directory and proves:

- no task or prompt exists before explicit onboarding actions;
- a setup task is inert until Connect;
- the real provider can connect using its existing account state;
- an existing local project is added through the first-run Project step;
- Finish setup persists onboarding completion without submitting a prompt;
- one explicit user prompt receives a real provider response;
- restart restores the provider task, project and event history without autostart.

Fixture-only onboarding remains regression coverage, not A04 evidence.

## A10 - cross-platform visual/accessibility/save-picker

Automatic foundation: `.github/workflows/ui-acceptance.yml`.

The hosted Linux lane renders the actual GPUI app under a private X11 display,
exercises theme paint and keyboard interaction, captures synthetic screenshots,
and runs save-picker ownership tests. The normal platform matrix separately runs
save-picker ownership tests on Linux, macOS and Windows.

Those checks do not by themselves close A10. Final evidence is submitted through
`.github/workflows/live-ui-acceptance.yml` from interactive self-hosted
platform sessions.

Required live runner labels:

- macOS Apple Silicon: `self-hosted, macOS, ARM64, synara-live-ui`
- Windows x64: `self-hosted, Windows, X64, synara-live-ui`
- Linux x64: `self-hosted, Linux, X64, synara-live-ui`

Each platform evidence file uses format `synara-a10-live-v1`, is tied to the
exact candidate SHA, and must report successful light/dark/scaled visual checks,
keyboard navigation, native accessibility names/roles/states, focus after native
dialogs, an active screen reader, successful/cancel/no-overwrite save-picker
behavior, plus hashed artifacts for light/dark screenshots, the accessibility
log and save-picker log.

The validator rejects platform mismatches, incomplete checks, path traversal,
missing/duplicate artifact kinds and digest changes. The workflow uploads only a
sanitized summary containing environment labels and artifact hashes, not the
screenshots or raw accessibility/save-picker logs.

A10 closes only after accepted evidence exists for all required target platforms.
