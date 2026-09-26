# Parity continuation verification - batch 36

Date: 2026-09-26

## Accepted

### A05 Authenticated browser login/session/popup acceptance

Authenticated browser acceptance run `36241446487`, job `108402605713`,
completed successfully for exact candidate
`163d59cf1eaba301e413f1d02848a4d4cb397f69`.

The dedicated lane passed:

- the authentication popup flow/partition authority unit contract;
- a real WebKitGTK/Xvfb authentication journey;
- cookie-backed continuity inside one authentication flow;
- capture/review before popup destination navigation;
- same-flow popup ownership with opener callback and original form POST;
- isolation from Manual browsing and a separate authentication flow;
- live-session retention while a related authentication tab remains open;
- final authentication-profile cleanup after the last flow tab closes;
- browser formatting and strict Clippy;
- exact candidate/source identity checks.

This closes A05. D2 remains OPEN because M07, M09, M11 and M12 are independent
browser/WebMCP product gaps.

## Acceptance infrastructure added

### A01 Live microphone + ChatGPT transcription

A01 is still OPEN. The repository now has
`.github/workflows/live-voice-acceptance.yml` plus the ignored
`live_microphone_chatgpt_transcription_end_to_end` test.

The lane intentionally requires a self-hosted runner labeled
`synara-live-voice` with:

- a physical/default microphone and operating-system permission;
- Codex installed and authenticated through ChatGPT;
- explicit manual workflow dispatch against an exact reviewed commit.

The test captures four seconds from the real default microphone, validates the
resulting WAV, obtains ChatGPT authentication through the real Codex app-server,
uploads only to the official ChatGPT transcription origin, and requires a
non-empty transcription. It prints only the transcript byte count, never the
transcript text.

No live run has been claimed yet, so A01 remains OPEN.

## Inventory effect

- A05 accepted: acceptance/integration remaining 5 -> 4.
- Shipped feature slices 78 -> 79.
- Execution total 20 -> 19.
- Broad parity gates remain 20 OPEN because D2 is still incomplete.
