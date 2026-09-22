# Native development build notes

This is a development build, not a published or signed production release.
These notes are bundled with the application. No release date is asserted.

Conversation tools
- Debug workflows record task-local phases and evidence, with explicit draft instructions.
- PR Fix reviews unresolved comments before adding instructions to an unsent task draft.
- Inline file comments preserve the selected range, file hash and exact excerpt.
- Thread recap reviews visible source and the destination model before separate, bounded generation and caching.

Ownership
- Coding agents use generic ACP. Direct model inference is a separate runtime.
- Import and continuation create reviewed conversations without transferring approvals or secrets.

Distribution
- Automatic update checks and installation are unavailable in this build.
- Production signing, endpoint selection and platform replacement remain unconfigured.
