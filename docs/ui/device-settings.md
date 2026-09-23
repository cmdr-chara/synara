# Native Device and Settings

Source checkpoint: September 21, 2026, `astra/device-settings`, based on
`980d86b59a1f06636a55aa0b75b71eef41fd3841`. Source implementation does not establish
hardware, input, macOS, Windows or native capture acceptance.

Focused Linux `cargo check` passes at `744830e`. The latest applicable focused suites
contain 40 distinct passing tests, including shutdown/deletion and saved Device-tab
regressions. The [executed validation ledger](../verification/device-settings-session.md)
records exact revisions and earlier failures. These are not hardware or GUI acceptance.

## Device setup and supported targets

Open **Settings > Device / capture**. Choose Android / ADB or Apple Simulator.
For Android, select the **trusted installed SDK adb executable** using the native
file picker. No executable is downloaded, installed or discovered through a
project's PATH. The path is a preference, not a permission grant. Selection alone
runs nothing. Open Device in the Environment menu or command palette, then Refresh.
The ADB executable can execute code on the host: do not select an untrusted file.

| Target | Source implementation | Explicit limits |
| --- | --- | --- |
| Android USB device | ADB discovery, authorization/offline status, PNG capture, probed tap/swipe/key input | USB debugging and RSA authorization must be configured by the user. No physical-device shutdown, boot, text injection or automatic pairing |
| Running Android emulator | ADB discovery, PNG capture, probed input, confirmed emulator shutdown | Cold boot and AVD enumeration are not implemented. Start the emulator externally |
| Android network target | Listed when ADB reports it | Physical/emulated kind stays unknown without USB or emulator evidence. No automatic network pairing or reconnect to guessed addresses |
| iOS Simulator on macOS | Installed-runtime discovery, boot, confirmed shutdown, simctl PNG capture, user-triggered HTTP(S) URL opening and installed-app launch | Requires Xcode and an available iOS runtime. No Apple input API, app installation, physical iOS device support or private framework linkage |
| Other Apple runtimes | Explicit unsupported rows when simctl reports them | tvOS/watchOS/visionOS and unavailable runtimes are not exposed as working iOS targets |
| Apple target on non-macOS | Explicit unsupported setup state | No helper invocation |
| Desktop AppSnap/window capture | Not implemented | No claimed screen-recording permission, window enumeration or shortcut support |

All Apple command construction/parsing is in
`crates/synara-runtime/src/device_tools/apple.rs`. Portable domain logic keeps
`DeviceDescriptor`, `DeviceId`, `DeviceInput`, `DeviceInputConsent` and frame
validation. The existing protocol remains available for a future native helper.
These command adapters are not a claim that that entire helper protocol is complete.

## Viewer and authority

Select a reported target, then Capture. Frames are bounded PNG snapshots, not a
fabricated video feed. The workspace image decoder applies dimension and allocation
limits and reuses `DeviceFrame` validation. Portrait/landscape/square metadata comes
from decoded pixels. The aspect-fit viewport follows pane resizing without changing
the device resolution. Letterbox clicks never become device coordinates.

For a selected, booted iOS Simulator, enter an HTTP(S) URL and choose **Open URL**,
or enter the bundle ID of an already installed app and choose **Launch installed
app**. Both actions use `simctl` with bounded output, deadline and cancellation.
Input is validated before a helper starts. The previous screenshot is cleared on
success; choose Capture to inspect the new screen. These actions grant no input
authority and have not been exercised against a live macOS simulator in this sprint.

Input is off until the user explicitly enables it on the selected ready Android
target. A real `/system/bin/input` executable probe must succeed first. The grant
is bound to that target and helper, is not serializable, and is never restored.
Tap, preset vertical swipes, Home/Back/Enter/Backspace buttons and focused arrow-key
input use fixed command names and integer arguments. User text, shell fragments and
arbitrary keycodes are not accepted. Input requires a capture at most 10 seconds old.
This checks a helper capability, not successful physical-device input acceptance.

Selection, Refresh, errors, configuration changes, hiding, disconnect, Escape and
orientation changes revoke authority. Late replies carry an epoch and cannot replace
a new target's frame or grant. Successful discovery retains missing targets as stale
rows. Failed operations retain the list for inspection but revoke its actionable state.
Refresh is the explicit reconnect/recheck operation, not a silent pairing action.

Optional capture every two seconds starts only after an explicit Capture and only
while the viewer is visible. Hidden viewers cancel pending commands and release the
frame. Returning requires Capture again. Zen does not destroy tasks, sessions or the
device itself. Shutdown requires a second confirmation and only targets supported
simulators/emulators. Synara never sends `adb kill-server`. ADB may manage its normal
shared server, which needs platform acceptance alongside process-tree ownership.

Owned helper commands have output limits, a deadline, cancellation and bounded
shutdown/reaping. Closing the app cancels device work. No captures are written to
disk or attached to chats automatically. **No device tools are registered with agents**:
the real agent-tool bridge is absent, rather than represented by pretend capabilities.

## Settings inventory for this session

| Area | Actual implementation or retained behavior | Remaining scope |
| --- | --- | --- |
| General | Persisted restore-last-chat preference, valid saved selection only, no arbitrary archived fallback. Existing provider, sorting, sidebar and Environment defaults retained | OS login/startup registration and multi-window recovery |
| Chat | Existing send-on-Enter and timestamp preferences. New recent-attachment-list visibility never removes pending files | More composer/thread policies and agent-supported queue/steer |
| Agents | Existing registry/profile manager, current connection state, advertised authentication actions and capability-inspector link | Provider-specific account claims are not inferred |
| Models/config | Existing real configuration control owner is reused in Settings | No static model list, invented effort options or automatic prompt sending |
| Keybindings | Editable effective native navigation bindings, aliases/case normalization, conflict checks against defaults, reserved editing/Space/Zen shortcuts, restore defaults, palette and Help discovery | Editor, terminal, other application shortcuts and arbitrary command rebinding |
| Appearance | New higher-contrast text/separators. Existing typography, terminal size, density, width, reduced motion, material and profile import/export retained | No guaranteed contrast ratio for arbitrary wallpaper/glass. Native title/chrome preferences remain open |
| Notifications | Off-by-default background-completion preference, content-free messages, explicit test via available desktop transport | Linux notify-send/D-Bus or macOS osascript availability is not visible-delivery proof. Windows adapter absent |
| Privacy/security | Actual controller secret-store state, explicit connection-trace clearing, discard device frame/authority, data/retention explanations | OS secret-store implementation, backup UI and global secure erasure |
| Remote | Existing pinned SSH enrollment/settings and System-tools entry retained | No new remote-device helper or remote OS permission inference |
| Device/capture | Real helper settings, explicit setup/permission instructions, transient input authority, capture interval preference | Hardware matrix and AppSnap remain open |
| Archived data | Typed-confirmation archived-thread deletion with prompt reservation, serialized session close and atomic archived-state check. Task-owned preferences removed with transcript | No automatic retention expiry or archived-project bulk deletion. Shared promoted Hub knowledge, external history and backups remain |
| Profile/usage | Existing local activity and actual reported token/context values retained | No inferred plan, subscription, quota or account limit |
| Integration sessions | Checked against integration head `980d86b` before publication | Browser/Automations/PR and Plugins/Skills/MCP session work had not landed at that checkpoint. Their placeholders were not overwritten or falsely integrated |

Navigation bindings use Primary (Control or Command), digits, F1 through F12, or
Primary+Alt+letter except T/Z. Shift is optional. Reserved native keys remain owned
by their text/editor/Space/Zen handlers. Unknown historical binding entries remain
stored but inactive. A failed save keeps the current active settings unchanged.

Higher contrast strengthens secondary text and separators without replacing Glass
with an opaque panel. New controls use native button roles, labels and keyboard
activation. Device frames have a focusable, labeled group and explicit input controls.
Native focus, screen-reader and contrast acceptance still require actual journeys.

## Deletion and privacy details

Select another chat before deleting an archived thread. Finish pending imports,
follow-up edits, saves and active work. The controller reserves the task against
new prompts and serializes session close with setup. Close failure or timeout keeps
the stored data. Only that session is closed, not the shared agent process. SQLite
acquires its writer lock before the archived-state check and performs deletion in
one transaction. A concurrent restore cannot slip between the check and deletion.

Deletion is not secure disk erasure. Database free pages, OS snapshots, exported
backups, external agent histories and previously promoted/shared Hub knowledge may
retain copies. Workspace files are never deleted. Diagnostics are bounded redacted
in-memory metadata, not conversation deletion. Clearing affects a shared connection
and later protocol traffic can add new entries.

## Primary platform references

- Android SDK ADB command reference: https://developer.android.com/tools/adb
- Apple Xcode simulator documentation: https://developer.apple.com/documentation/xcode/interacting-with-your-app-in-device-hub
- Installed Xcode `xcrun simctl help`, `help io` and `help boot` remain the authority
  for the actual host's supported command behavior and must be recorded in acceptance.

See [implementation and validation receipt](../verification/device-settings-session.md).
