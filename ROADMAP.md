# Roadmap: Synara in Rust

Port Synara (TypeScript monorepo) to a pure Rust application using [rust-anything](https://github.com/erkamyaman/rust-anything), one unit at a time, proven against the original's recorded behavior.

**Clean-room rule (ADR-001):** Synara's TS code is MIT-licensed to T3 Tools Inc / Emanuele Di Pietro, so the Rust port must contain no T3-authored code. Ports are written only from recorded behavior (`cases.jsonl`), exported signatures and contract types — never by reading or translating the original source bodies. See [rust/DECISIONS.md](rust/DECISIONS.md).

**Attribution:** the Rust port is licensed MIT under `rust/` — Copyright (c) 2026 Emanuele Di Pietro and cmdr-chara ([rust/LICENSE](rust/LICENSE)).

Full generated plan with every unit, wave, readiness score and risk: [rust/plan.md](rust/plan.md) (`rust-anything analyze --whole-app`, 2026-11-20).

## Analysis snapshot (phase 0 done)

- **881 units** across 12 packages, ~874k analyzed source lines (2,405 source files, 1,609 test files)
- Readiness: **572 ready · 217 caution · 5 blocked · 87 keep** (UI stays in source language for now)
- Waves 0–9 per package; a unit's imports are always in earlier waves

### Units by package (port order)

| #   | Package            | Path                  | Units | Waves |
| --- | ------------------ | --------------------- | ----: | ----- |
| 1   | @synara/contracts  | `packages/contracts`  |    79 | 0–9   |
| 2   | @synara/profile-ui | `packages/profile-ui` |     9 | 0–2   |
| 3   | @synara/shared     | `packages/shared`     |   142 | 0–4   |
| 4   | @synara/analytics  | `apps/analytics`      |    17 | 0–6   |
| 5   | @synara/api        | `apps/api`            |    17 | 0–6   |
| 6   | @synara/desktop    | `apps/desktop`        |   105 | 0–6   |
| 7   | @synara/marketing  | `apps/marketing`      |    24 | 0–2   |
| 8   | @synara/profiles   | `apps/profiles`       |     7 | 0–3   |
| 9   | @synara/web        | `apps/web`            |   198 | 0–6   |
| 10  | @synara/cli        | `apps/server`         |   240 | 0–9   |
| 11  | @synara/scripts    | `scripts`             |    39 | 0–7   |
| 12  | @synara/monorepo   | `.`                   |     4 | 0–4   |

### First 10 units (start_here)

1. `packages/shared/src/toolOutputSummary.ts` — 69 ln, readiness 97
2. `apps/desktop/src/backendIssueDetector.ts` — 36 ln, readiness 96
3. `packages/shared/src/githubRepository.ts` — 72 ln, readiness 85
4. `apps/web/src/providerOrdering.ts` — 59 ln, readiness 80
5. `packages/shared/src/projectFolders.ts` — 114 ln, readiness 100
6. `apps/server/src/codexTurnInput.ts` — 42 ln, readiness 100
7. `packages/shared/src/threadHierarchy.ts` — 48 ln, readiness 100
8. `apps/desktop/src/windowsShellIco.ts` — 132 ln, readiness 100
9. `apps/desktop/src/backendStartupBlock.ts` — 134 ln, readiness 93
10. `packages/shared/src/agentMentions.ts` — 134 ln, readiness 98

### Blocked (needs a human decision, 5)

- `apps/desktop/scripts/`, `apps/server/src/provider/`, `scripts/lib/`, `scripts/release-smoke.ts` — dynamic code (eval / computed imports)
- `apps/web/scripts/` — vite, no Rust equivalent

### Key dependency decisions (fit)

- **Good**: `ws`→tokio-tungstenite, `smol-toml`→toml, `undici`→reqwest
- **Partial**: `hono`→axum (middleware by hand), `zod`→serde+garde (error messages differ), `yaml`→saphyr/serde_yaml_ng (no comment-preserving rewrite), `typescript`→oxc (parse only, no typecheck)
- **Unknown/none (needs ADR)**: `effect` (285 units!), `zustand` (34), `electron` (17), `@effect/platform-node` (16), `jose` (11), `drizzle-orm`, `@anthropic-ai/claude-agent-sdk`, `@hono/node-server`, `next`, `node-pty`, `@xterm/headless`, MCP/agent SDKs, and more — see plan.md
- **Keep in source language**: `vite` dev tooling; 87 UI units (`apps/web` React, `apps/marketing`, `apps/analytics` UIs) — replacement decided per-decision, default: rebuild in gpui-kit (gpui-fast) or keep behind the front door

## Phases

- **Phase 0 — Analysis** ✅ done (this file + `rust/plan.md`)
- **Phase 1 — Decisions**: interview rounds → `rust/DECISIONS.md` ADRs (shape, scope, equivalence bar, dependency crates, targets, cutover)
- **Phase 2 — Record**: `app record` on entry points; `app replay` must hit 100%
- **Phase 3 — Front door**: `rust/app` — axum for the server, gpui-kit (gpui-fast) shell embedding the web UI via native webview for desktop; fallback to original for everything not ported
- **Phase 4 — Port by waves**: per unit: bridge → record → `src/port.rs` → build → diff N/N → bench → gate → `app wire`; fallback rate only decreases
- **Phase 5 — Lift & cutover**: fallback zero → `app lift` → pure Rust binary → `rust/cutover.sh`

## Status

| Phase                | State                                                          |
| -------------------- | -------------------------------------------------------------- |
| 0 — Analysis         | done — 881 units planned                                       |
| 1 — Decisions        | done — ADR-001 clean-room, ADR-002 T3 cases-only rule          |
| 2 — Record           | done — app record 179/179 + cases per bridged unit             |
| 3 — Front door       | done — `rust/app` axum (179/179 replay, 100% fallback) + `rust/shell` gpui-kit (gpui-fast) webview shell |
| 4 — Port (789 units) | in progress — 402 ported+gate-green; ~45% units unbridgeable   |
| 5 — Lift             | pending                                                        |

## Provenance (T3Code audit)

`rust/T3CODE-PROVENANCE.md`: deep audit vs pingdotgg/t3code @ `d15210cd`. All committed ports pass clean-room checks except 6 line-derived crates scheduled for cases-only rewrite (worklog, storeeventreducer, keybindingrules, shell, terminal-links, markdown-links). trycua/cua-derived code is accepted as-is per operator decision.
