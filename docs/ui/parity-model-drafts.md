# Provider favorites, durable drafts and Chat behavior

Date: September 20, 2026 (Europe/Rome).

## Implemented slice

Provider-icon tabs and a Starred tab now filter native model choices without changing their controller indices. Advertised model favorites persist by agent, selector and value. Starring never selects a model, starts an agent or sends text. Explicit Connect and advertised mode/config choices are available in the same picker. The model list uses compact rows, native search, saved UI fonts, the existing 150 ms entrance and reduced motion. The two added Tabler star paths match the icon family used by Electron 9488759.

Drafts are stored per task, separately from messages and settings. Writes are debounced, serialized and coalesced. Late loads and user-message echoes cannot erase newer edits, including identically retyped text. Drafts restore without starting agents. Deleting a task removes its draft in the same transaction. Closing drains writes in a native modal, supports cancellation, and keeps text available on a save error.

Chat settings now edit Enter-to-send and message timestamps, persist them and restore only Chat defaults. Shift+Enter adds a line. With Enter-to-send disabled, Ctrl+Enter or Command+Enter still sends. Settings apply after successful persistence. No permission policy, protocol schema or dependency was changed.

## Verification evidence

Platform: Ubuntu 24.04 x64, Rust 1.98.1, locked GPUI, private X11/Xvfb with owned SQLite/project data and ACP fixture agents.

The candidate generated at `a1fe9d7fe853a971160b36555c39f2e1caccb38b` passed [storage/settings, draft/menu/control/input/close unit tests, changed-package strict Clippy, app/fixture build, native model/draft, native Chat behavior, Studio creation and guarded-close journeys](https://github.com/cmdr-chara/synara/actions/runs/35476586720/job/105986775858). That run as a whole FAILED its combined legacy session/search step. It is not reported as an overall pass.

The same application generator was reused unchanged for `1828d1e9366e9ea71406b0e37027abb9ae24594a`. Only the legacy test input targeting was corrected: explicitly select the owned composer/search editor after native clipboard and popup operations. The exact text, event and release-only assertions remain. [This focused follow-up](https://github.com/cmdr-chara/synara/actions/runs/35477290881) passed both native session-control and filtered-search suites, formatting, scope tests and application/fixture build. Already-passing application unit/lint/new native journeys were not repeated in this follow-up.

The earlier restart test incorrectly launched with --workspace for the original project, overriding saved chat selection. It now requests restoration explicitly and checks task-scoped events for the selected chat. This was a harness correction, not a claim that a product restart defect was fixed.

## Reference and remaining gaps

Reference: Emanuele-web04/synara at `948875954f432978eab7dd5fa44c3028b8d99a81`. The requested 73-image archive was retried through local and independent file-access paths. The local host and image access continued to return transport timeouts, and the file-library searches did not recover the archive. No 73-screen inspection or pixel-level visual acceptance is claimed. Native test captures are retained with the workflow artifacts for seven days.

D4/D8/F2/I6/I10 advanced but remain open as whole gates. Favorites currently save model identity, not complete model-plus-effort trait presets, and only currently advertised choices can be applied. Full browser embedding, approval-policy parity, Studio output/image trees, Kanban task creation, Spaces, automation scheduling, attachments/voice, richer transcript actions, remaining settings and cross-platform GUI acceptance are still open. Native model-provider authentication and actual OS IME/screen-reader journeys were not tested.

## Publication integrity

The source delta is exported only after the selected checks pass. It is applied through the existing branch/root-guarded source publisher. Final publication must remove the temporary generator workflow and verify these source blob identities. No changes to main, archived history, PRs or releases are authorized by this checkpoint.

```text
b19bb3b5a40f2d61dc429ff958cd7792503868ec  crates/synara-app/assets/icons/model-picker-manifest.json
f8a43388c6d008fbeb96ba9bc1b18f7a917b84bc  crates/synara-app/assets/icons/tabler/star-filled.svg
488c0e7b8e554ee57e6e2fe15ce99fd19491f571  crates/synara-app/assets/icons/tabler/star.svg
1126d383375ec02f7ea2ee3e8612686d3bc36e0f  crates/synara-app/src/input.rs
8a336663650dc84ab15e785adeace7fe1ce406d4  crates/synara-app/src/input/policy.rs
296384226575fb1d10640c77c45328af9ef61b4a  crates/synara-app/src/shell.rs
260bf5d2a56f695f501bf1f1be3c71dfaabf3cb8  crates/synara-app/src/shell/chrome.rs
2f679ea2e8afb7e4c87f1216042d90db792f3a8a  crates/synara-app/src/shell/controls.rs
7123a6ae11a69420e9aaffa92a49a0f63f0908a9  crates/synara-app/src/shell/drafts.rs
95226d05eec2b911bcc3c6833ca2fb79c9c5467c  crates/synara-app/src/shell/messages.rs
2d2ef6eead2fe467c7c5dbbb9258aaf513b8bdb8  crates/synara-app/src/shell/navigation.rs
f87125267ba0f37c5ca86d442360f0500f109109  crates/synara-app/src/shell/settings.rs
98beacb6ce5f6fef0f612c771bea5b627011f1f6  crates/synara-app/src/shell/settings/chat.rs
7d0b6661a000b8583717d5bfd5dafe13d889eee2  crates/synara-app/src/ui/icons.rs
6b23da2726959b63a333fcf273223f957b63c0b0  crates/synara-app/src/ui/menu.rs
085f5c349a3ae25fa711d398d926221890e0cab8  crates/synara-app/src/ui/menu/models.rs
c7e82fa85a5417230ed8ba390de2baa4d9547bd8  crates/synara-workspace/src/settings.rs
2a85e47897c7df1bab72e1156d1e40d77eeb1d82  crates/synara-workspace/src/settings/chat.rs
d415ec6946b74c1971bd20433d7b23f5346516fb  crates/synara-workspace/src/storage.rs
a5d020f6a26521d60e1a5118441c1766c3a2d3e7  crates/synara-workspace/src/storage/chat_preferences.rs
ddb818dc80ad62301762ed752b6f76c8ab4ed612  scripts/native_chat_behavior_smoke.py
577c82b5d681c67a7ad9008a2bd72827be853cf9  scripts/native_controls_smoke.py
c22f208a737b8d8306eb74877ed858808b3ffa9d  scripts/native_model_draft_smoke.py
9977fe9712a2998589aeb61474cb15b11f881848  scripts/native_picker_search_smoke.py
3e71994ed59f688ebd4173bd7f7b8fa1c4afbfaf  scripts/native_smoke.py
7b83736fd4fe31b0e7a099bfb59236415c1684e0  scripts/native_ui_scope.py
a28c7c78532ed03df17aa9bd72711570fb8dc257  scripts/test_native_ui_scope.py
```
