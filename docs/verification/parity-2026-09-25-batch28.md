# Parity batch 28: binary document pipeline closure

## M29 complete

The binary document attachment/viewing pipeline is complete at the product-feature level.

### Supported bounded document snapshots

Explicit attachment intake supports:
- PDF;
- DOCX;
- ODT;
- ODP;
- ODS;
- PPTX;
- XLSX.

Every binary format is revalidated from its selected bytes before preview or prompt projection.
The original bytes remain task-owned and local until an explicit export.

### Inert extraction boundary

Office formats expose only bounded inert text:
- DOCX opens only `word/document.xml`;
- ODT/ODP/ODS open only bounded `content.xml`;
- PPTX opens only bounded numeric slide XML;
- XLSX opens only bounded numeric worksheet XML plus the optional shared-string table.

Relationships, macros, scripts, external resources, media and embedded objects are not followed
or executed. Spreadsheet formula expressions are never evaluated; only visible or cached values
are projected. Slide/sheet/page sections are labeled so prompt context retains document position.

### PDF viewer depth

Studio uses an immutable PDF snapshot and fixed system Poppler helpers for:
- bounded page count and page rendering;
- paging, fit and zoom;
- one-page text extraction/copy;
- labeled first-12-page document text extraction/copy;
- safe HTTP(S)-only link annotation inspection and explicit opening;
- read-only disclosure of no form / AcroForm / XFA / unknown form technology;
- explicit original-file export through the no-overwrite writer.

PDF actions and scripts are never executed. Form fields are not presented as interactive because
the current fixed helper contract does not expose a safe field-level mutation/submission owner.

### Remaining scope

M29 is complete. S18 remains open only for safe field-level PDF interaction beyond the existing
text/link/form-metadata workflow. D13 and A10 remain OPEN for comprehensive failure journeys,
native helper packaging and cross-platform save-picker/viewer acceptance.
