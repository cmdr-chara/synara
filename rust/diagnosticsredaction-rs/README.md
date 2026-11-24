# diagnosticsredaction-rs

FILE: diagnosticsRedaction.ts Purpose: Technical-only diagnostics payloads and local free-text redaction. Telemetry uses the fixed-category helpers; text redaction alone is not a content boundary.

napi-rs bridge for `packages/shared/src/diagnosticsRedaction.ts`.

## Functions

- `sanitizeDesktopUpdateFailureReason` (`packages/shared/src/diagnosticsRedaction.ts:15`)
- `classifyDesktopUpdateFailure` (`packages/shared/src/diagnosticsRedaction.ts:22`)
- `sanitizeDiagnosticError` (`packages/shared/src/diagnosticsRedaction.ts:95`)
- `sanitizeDiagnosticCrash` (`packages/shared/src/diagnosticsRedaction.ts:192`)
- `diagnosticErrorFingerprint` (`packages/shared/src/diagnosticsRedaction.ts:221`)
- `diagnosticPayloadKind` (`packages/shared/src/diagnosticsRedaction.ts:237`)
- `redactDiagnosticText` (`packages/shared/src/diagnosticsRedaction.ts:408`)