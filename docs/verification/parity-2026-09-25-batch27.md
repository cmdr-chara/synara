# Parity batch 27: durable turn-attributed Studio output history

## M30 complete

Studio's historical output/version lifecycle is now complete at the product-feature level.

### Automatic output snapshots

When a completed tool reports a changed file in the currently open Hub, Synara now
runs a separate bounded capture pass before the ordinary Library refresh. The capture:
- reconstructs output ownership from durable tool/turn events;
- considers only visible reported files with exact source-task and source-turn metadata;
- reads at most 16 recent candidates per pass;
- snapshots only bounded UTF-8 text files up to 128 KiB;
- rejects binary/non-UTF-8 content rather than coercing it;
- deduplicates unchanged content for the same path.

Each stored version carries the reporting task, reporting turn and report timestamp.
The native version picker displays the reporting turn when available.

### Durable lifecycle

The existing durable ledger survives restart and remains capped at 12 entries / 1 MiB.
It now also supports:
- exact-snapshot export through the no-overwrite writer;
- exact-snapshot pin/unpin;
- automatic retention that evicts only unpinned history;
- explicit failure when pinned history fills the retention budget;
- explicit two-step clear for one path;
- manual preview-driven capture for ordinary text files.

Pinning protects against automatic retention only. Explicit Clear remains the deliberate
destructive action. No history operation rewrites or deletes the workspace file.

### Ownership and stale-state behavior

Exact snapshot identity includes task, visible path, text, capture timestamp, pin state and
optional reporting provenance. Export and pin/unpin reopen the Hub-owned ledger and require
the exact snapshot to still exist. Cleared, evicted or stale snapshots fail closed.

Automatic capture is triggered only from the existing open-Hub tool-finished owner. It is
separate from Library listing, so merely browsing the Library does not mutate history.

### Focused regression

The workspace regression records two completed tool turns that report the same text file,
captures version one and version two with exact turn attribution, verifies an unchanged
second sweep creates no duplicate, and checks retained provenance on both versions.

M30 is complete as a product feature. Broader live-provider/platform evidence remains part
of the Studio acceptance gate rather than the execution-feature count.
