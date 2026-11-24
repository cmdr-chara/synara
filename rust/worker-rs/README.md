# worker-rs

FILE: apps/analytics/worker.ts Purpose: Cloudflare Worker ingest + dashboard API for Synara Beta diagnostics. Endpoints:   POST /v1/events   — NDJSON diagnostics events -> D1 `events` table   POST /v1/reliability-events — anonymous reliability receipts -> D1 daily totals   POST /v1/crash    — permanently retired; body is never consumed   POST /api/login   — password login -> signed session cookie   GET  /api/*       — dashboard reads (session cookie required)   GET  /healthz     — liveness   GET  /*           — dashboard SPA (session required; /login is public)  The worker is deliberately allowlist-shaped: unknown event names, unexpected field types, and oversized payloads are dropped rather than stored.

napi-rs bridge for `apps/analytics/worker.ts`.

## Functions

- `normalizeProductEvent` (`apps/analytics/worker.ts:283`)
- `normalizeEvent` (`apps/analytics/worker.ts:550`)
- `signSession` (`apps/analytics/worker.ts:798`)
- `verifySessionCookie` (`apps/analytics/worker.ts:818`)
- `parseApiFilters` (`apps/analytics/worker.ts:1013`)
- `compareVersionsDesc` (`apps/analytics/worker.ts:1100`)
- `releaseHealth` (`apps/analytics/worker.ts:1197`)
- `rangeDays` (`apps/analytics/worker.ts:1271`)
- `purgeLegacyDiagnosticContent` (`apps/analytics/worker.ts:2444`)
- `purgeLegacyDiagnosticDumps` (`apps/analytics/worker.ts:2508`)