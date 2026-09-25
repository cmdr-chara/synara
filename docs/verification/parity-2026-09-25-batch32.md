# Parity continuation verification - batch 32

Date: 2026-09-25

## Scope

This batch advances acceptance infrastructure without treating automated build evidence
as live platform acceptance.

- Development macOS packaging now emits a real `Synara.app` bundle layout instead of
  a bare executable tarball.
- The development app bundle declares `NSMicrophoneUsageDescription` with explicit
  user-initiated recording wording. It remains unsigned and development-only.
- A dedicated cross-platform workflow builds the native application for Linux x64,
  macOS arm64 and Windows x64, runs the package/inventory self-tests, creates the
  deterministic development package and uploads candidate/environment evidence.
- The workflow pins the exact candidate and verifies that acceptance steps do not
  modify tracked source.

## Acceptance impact

- **A02 remains OPEN.** The package now contains the macOS microphone permission
  declaration needed for a real permission prompt, but signed bundle/notarization
  and live microphone permission behavior still require macOS acceptance.
- **A03 remains OPEN.** Windows now has a first-class native build/package lane, but
  live microphone privacy behavior and installed-package acceptance are still needed.
- **A10 remains OPEN.** The workflow supplies cross-platform build/package evidence,
  not visual, accessibility or native save-picker interaction proof.
- **A08 remains OPEN.** These artifacts are explicitly development-only and are not
  a trusted signed update feed or rollback package.

## Focused verification

The package helper contains a regression test that opens the generated macOS tarball,
parses `Info.plist`, checks the microphone usage string and verifies executable mode.
The pushed workflow is the deciding evidence for cross-platform compilation/package
creation on hosted Linux, macOS and Windows runners.
