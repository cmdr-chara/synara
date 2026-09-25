# Parity continuation verification - batch 33

Date: 2026-09-25

## Accepted

### A02 macOS microphone packaging/permission acceptance

GitHub Actions run `36169108248`, job `108184152199`, completed successfully
on hosted macOS arm64. The exact candidate `2420127b45a6fe8c00ed5e49bd800534a58c59a9`
passed:

- the native voice regression tests;
- the native application build;
- dependency inventory and deterministic development package creation;
- native `plutil` validation of the generated `Synara.app/Contents/Info.plist`;
- an explicit `NSMicrophoneUsageDescription` check;
- packaged executable-presence/executable-mode validation.

The only later change before this acceptance receipt touched
`scripts/remote_native_smoke.py` for A07 and cannot invalidate this package/voice
evidence.

### A03 Windows voice/package acceptance

The same run, job `108184152635`, completed successfully on hosted Windows x64.
The exact candidate passed:

- the native voice regression tests;
- the native application build after the Windows file-identity portability fixes;
- dependency inventory and deterministic package creation;
- expansion of the produced ZIP;
- presence of `synara-app.exe`;
- exact Windows target and development-only manifest validation.

## Still open

A01 remains open because it requires the real microphone + ChatGPT transcription
end-to-end journey. A07 remains open pending the full SSH worktree/search/native-UI
acceptance journey. A10 still requires visual/accessibility/save-picker proof.
