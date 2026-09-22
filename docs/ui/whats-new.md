# What's New and local build history

Settings > What's New shows the compiled Cargo package version, bundled
**development** notes, update availability and the builds observed in this local
installation. A notice appears when the version or bundled-note fingerprint
differs from the last acknowledged build. Mark these notes read is explicit and
durable. Merely opening the page does not acknowledge them.

History is bounded to 24 observations. Its timestamps are first-observed local
Unix milliseconds, not publication dates, verified installation receipts or
proof that a platform updater ran. Version changes in either direction are
recorded without guessing whether they were upgrades or rollbacks. A note change
under the same development version is also visible.

The page does not invent a release feed. It states that no verified published
release catalog is bundled. Automatic update checking, downloading, replacement
and verified rollback are unavailable because no production endpoint, signing
authority or platform replacement helper is configured. The existing
`synara-runtime::UpdateManifest::verify_signed` and no-clobber staging remain
unchanged. No network request, GitHub release or executable replacement is
performed by this feature.

Metadata is kept separately from tasks/transcripts/drafts under
`native-release-experience-v1`. Acknowledgements are revision checked. Stale
acknowledgements cannot hide notes for a newer build, repeated loading does not
add duplicates, restart preserves the acknowledgement, and malformed or
oversized history is reported without overwriting it. Close waits for a pending
history write. There is no automatic recovery that fabricates prior history.

## Verification and remaining depth

`releases::tests` covers acknowledgement, stale revisions, changes, rollback
observations, bounds, malformed history and SQLite reopen.
`native_sprint2_releases_smoke.py` covers the native page, explicit mark-read,
reload and restart with the conversation and normal draft preserved.

This is a **partial Releases capability**, not a complete production updater.
Verified published history, a configured release source, signing/transport and
platform update installation remain open. No release is authorized or created.
