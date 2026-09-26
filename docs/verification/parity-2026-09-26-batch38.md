# Parity continuation verification - batch 38

Date: 2026-09-26

## Accepted

### A08 Signed release feed/install/rollback package acceptance

GitHub Actions run `36243757479` completed successfully on exact candidate
`5e006f08d31a2ca7eea97e7b7d0c0d0052011cb6`.

Passing jobs:

- Linux x64: `108409041271`
- macOS arm64: `108409041457`
- Windows x64: `108409041403`

Every target:

- built the native release candidate and deterministic package;
- generated a release feed bound to the exact package and update manifest;
- keyless-signed package, manifest and feed with GitHub OIDC/Sigstore;
- verified the exact Synara workflow identity and OIDC issuer;
- verified the signed manifest before accepting update metadata;
- rejected a tampered manifest;
- rejected an unexpected workflow identity and issuer;
- rejected corrupt download bytes and removed the failed stage;
- rejected staged-artifact tampering while preserving the installed bytes;
- staged the exact signed artifact, reverified it at handoff, exercised package
  replacement in the owned test installation, then restored the rollback copy.

The three commits after the accepted candidate only add or refine A04/A10
acceptance harnesses and documentation; they do not change the release package,
runtime update implementation or signed-release workflow inputs. A08 therefore
remains valid on the resulting branch state.

## Scope boundary

A08 is the release-package acceptance gate. It does not close M28 or broad gate
D10. The current runtime still delegates final production running-executable
replacement and production endpoint/signing policy to the future product updater
lifecycle.

## Remaining acceptance work

### A01

The live voice lane now requires non-zero physical microphone input before it
uses real Codex ChatGPT authentication and the official ChatGPT transcription
origin. Transcript text is never printed. A01 remains OPEN until that real
environment runs successfully.

### A04

The real-provider lane now covers the complete first-run sequence: inert setup
task, explicit Connect, project registration, Finish setup, one explicit real
provider turn and restart without autostart. A separate hermetic GPUI foundation
lane exercises the same project/finish/restart mechanics with the owned ACP
fixture. A04 remains OPEN until a real pre-authenticated provider runner passes.

### A10

A hosted Linux foundation lane now owns real GPUI theme paint, keyboard behavior,
synthetic screenshots and save-picker ownership checks. A separate live evidence
validator requires exact-candidate macOS/Windows/Linux evidence with a real
screen reader/native accessibility API, light/dark/scaled visual review and
save-picker success/cancel/no-overwrite behavior. It uploads only a sanitized
summary and artifact hashes. A10 remains OPEN until the required live platform
evidence is accepted.

## Inventory effect

- A08 accepted: acceptance/integration remaining 4 -> 3.
- Shipped feature slices 84 -> 85.
- Execution total 14 -> 13.
- Broad verification gates remain 20 OPEN because D10 remains wider than A08.
