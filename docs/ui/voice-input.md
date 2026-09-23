# Native voice drafts

In a conversation, choose the microphone button to record, choose Stop to
transcribe, or Cancel to discard the recording. The composer explains before
recording that the clip is uploaded to ChatGPT for transcription. A successful
transcript is appended to the current unsent draft for review; it is never sent
as a provider message automatically.

Recording requires a selected task, a working microphone and operating-system
microphone permission. Transcription requires a ChatGPT-authenticated Codex
session. Clips stay in memory, are capped at 120 seconds and 10 MiB, and are
encoded as 24 kHz mono WAV. Authentication and upload requests are timed out;
the upload is restricted to the official ChatGPT HTTPS origin without redirects.
Cancel, task navigation and shutdown stop the active operation. If the task or
draft changes before the result arrives, Synara leaves the newer draft
untouched. An accepted transcript is written to the task draft store.

The Linux build and focused tests cover audio bounds, URL validation, stale
drafts and cancellation. A live microphone and ChatGPT upload were not tested
in this checkpoint. macOS/Windows compilation and packaged microphone
permission behavior remain acceptance work.
