# Decisions — Synara Rust port

ADRs for the Rust port. Numbered in order, never renumbered.

## ADR-001: Clean-room reimplementation — no T3 Tools code in the Rust port

Status: accepted (2026-11-20)
Context: The entire Synara TypeScript codebase is MIT-licensed to T3 Tools Inc and Emanuele Di Pietro (`LICENSE`). A line-by-line port is a derivative work and carries that copyright over; the goal is a rewrite containing no T3-authored code.
Decision: The Rust port is written **only from behavioral recordings (`cases.jsonl`), public interface signatures, and contract types** — never from the original source text. Specifically:

- The porting agent does NOT read the original `.ts`/`.tsx` implementation bodies. It may read: recorded cases, exported signatures, `packages/contracts` type shapes (as interface spec only), and public docs.
- No copying of structure, variable/function-internal names, comments, or expression from the TS sources. API/CLI/route names stay only where external compatibility requires them (names needed for interoperability are not copyrightable expression).
- `record`/`app record` outputs are behavioral facts — allowed as the spec.
- rust-anything's per-unit `file:line` risk notes may be read for **what** edge case exists (e.g. "string offset at index"), never for **how** the original implements it.
- Third-party imported code (`trycua/cua` files in `import-provenance.json`, `apps/desktop/patches/cua-driver`) is treated the same: interface + recorded behavior only.
- `gate` still applies to every unit; the equivalence bar comes from recordings, not code parity.

Consequences: Porting is slower and edge cases must be discovered through recording coverage rather than source reading. `record --inputs` and `--from-tests` become the primary tools for coverage. Any unit where recordings can't reach the needed behavior must be flagged for more recording inputs, not for reading the original.

## ADR-002: T3-derived units are (re)written cases-only

Status: accepted (2026-11-21)
Context: A provenance audit against `pingdotgg/t3code` @ `d15210cd` (the upstream Synara itself pins) found ~73 files ≥50% derived; only a few plan units touch them. The derived-file list lives at `t3code-derived-files.txt` in the repo root.
Decision: Any unit whose TS source appears on the derived list at ≥50% gets the strictest form of ADR-001: `port.rs` is written **solely from `cases.jsonl`** (enriched via `record --from-tests` and targeted `record --inputs` black-box probes), without ever opening the TS file. Applied so far:

- `worktreecleanup-rs` — rewritten cases-only (gate: 651/651)
- `proposedplan-rs` — ported cases-only, 2094 cases incl. JS TypeError fidelity (gate: 1997/1997)
- `providerruntime-rs`, `providerinstance-rs` — audited, already spec-derived; unchanged
- `terminal-links` (derived 62%) — will get the same treatment when bridged

Consequences: Every future unit is checked against the derived list before porting; derived ones get extra probe rounds until their spec is complete enough to pass `gate`.
