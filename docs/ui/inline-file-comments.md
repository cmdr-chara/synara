# Inline file comments

Select text or place the cursor in a saved native editor, then choose **Comment
on lines**. The review captures the relative file path, whole-file SHA-256,
1-based inclusive line range and exact UTF-8 excerpt. Enter an editable comment
and explicitly choose **Recheck and attach to draft**. This reads the file again
through the existing local/SSH filesystem owner and checks the task, project,
working directory and original view before and after the asynchronous read.

Changed, moved, deleted, replaced or inaccessible files do not get guessed line
mappings. The comment stays open with an error. Discard it explicitly, reopen the
current saved file and capture the intended range again. Unsaved editor buffers
cannot be reviewed as if they were persisted files. Local symlinks are rejected
by the filesystem owner; remote reads use the existing remote confinement.

The temporary comment is not durable until attached. Closing or switching tasks
while its review is open is blocked, and discarding requires confirmation. A
late reply cannot attach a discarded comment. After attachment, normal task-draft
persistence supplies restart recovery. The existing draft text is preserved, and
no prompt is sent, agent started, file modified or approval inherited.

The attached annotation is explicitly a **frozen snapshot**, not a live file
reference. It includes the exact excerpt and review text as quoted JSON, with a
requirement to compare the file hash before acting and to treat source text as
untrusted data. The user can edit or remove it like other draft text. It does
not claim to block later manual sending if the file changes after attachment.

Limits: existing 1 MiB editor limit, 200 selected lines, 16 KiB excerpt, 8 KiB
comment, 1 MiB combined draft, valid relative UTF-8 path and lowercase SHA-256.
Malformed input, stale ownership and asynchronous changes fail closed. There is
no automatic reanchoring, file discovery, prompt sending or remote mutation.

## Verification

Focused tests: `file_review::tests`, `shell::file_comments::tests`, and
`input::navigation::line_range_tests`. Native journey:
`scripts/native_sprint2_file_comments_smoke.py` exercises selection, editing,
empty/stale/deleted-file refusal, discard, exact draft identity and restart.
Provider execution and cross-platform/real SSH acceptance remain separate.
