# Parity continuation verification - batch 35

Date: 2026-09-25

## Accepted

### A06 Real macOS Simulator/device acceptance

GitHub Actions run `36201337810`, job `108288420785`, completed successfully
on hosted macOS arm64 for exact candidate
`cf9237f7e072b7b729cc1a84c593c74b0d289cfe`.

The runner used Xcode 15.4 and exposed real iOS CoreSimulator runtimes from iOS
17.0 through iOS 18.2. Synara's `DeviceTools` API selected an iPhone 15 Pro on
the iOS 17.0 runtime and passed all of these operations against CoreSimulator:

- discovery of the real simulator inventory;
- explicit boot and observation of the Ready state;
- PNG screenshot capture through the selected simulator;
- reviewed HTTP(S) URL opening;
- launch and termination of installed Mobile Safari;
- compilation of a fresh simulator-native test application;
- reviewed installation of that local `.app` bundle;
- launch and termination of the newly installed application;
- explicit simulator shutdown and observation of the Stopped state.

The preserved acceptance artifact reports the check sequence:

`boot, screenshot, open-url, launch-terminate-system-app, install-launch-terminate-local-app, shutdown`

The ignored real-platform test itself completed in 103.90 seconds with one pass
and no failures. The workflow also rechecked the exact candidate and left the
tracked source unchanged.

## Scope boundary

A06 proves the currently implemented Simulator discovery/lifecycle/capture/
URL/app operations on real macOS CoreSimulator infrastructure. It does **not**
claim M13-M16 are implemented. Live frame streaming, touch/swipe/typing and
hardware-button input, recording, and accessibility-tree/semantic targeting
remain separate product gaps.

## Still open

A01, A04, A05, A08, A09 and A10 remain open.
