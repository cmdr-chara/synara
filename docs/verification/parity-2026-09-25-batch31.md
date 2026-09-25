# Parity batch 31: safe PDF interaction closure

## S18 complete

Studio's immutable PDF snapshot now supports the remaining product-level PDF
interaction slice without enabling arbitrary PDF behavior:

- page and bounded document text remain inert and explicitly copyable;
- supported HTTP(S) annotations are inspected as data and open only after a user
  click;
- OCR is an explicit optional operation through the fixed system Tesseract helper;
- AcroForm fields can be inspected, with only ordinary text fields and bounded
  reported button/choice states exposed for local editing;
- password, file-select, rich-text/comb, push-button, multi-select, signature,
  XFA, unknown and read-only fields remain inspection-only or inert;
- saving a form always targets a new file through the existing no-overwrite
  export owner. The source snapshot is never modified.

The fill path re-inspects fields against the immutable source, validates every
edited name/value/option, XML-escapes a bounded XFDF payload, invokes only the
fixed system pdftk helper under private-directory/resource limits, re-inspects the
generated PDF and requires each requested value to round-trip exactly before
publishing it. PDF scripts, embedded files and SubmitForm/network actions are not
executed.

## Focused verification

- Commit `3deb89552d69f4c56fd7218877a53988a9a3e14e`: GitHub Actions
  **Roadmap and formatting** run `36149910462`, job `108120261002`, passed.
  This proves the repaired Studio PDF module parses and the pinned Rust formatter
  accepts the S18 app/workspace source.
- The workspace PDF module contains focused regression tests for bounded field
  parsing/XFDF escaping and for unsafe form-field flags remaining non-editable.
- The branch-wide native regression at the same commit reached Rust compilation
  but stopped on unrelated concurrent ACP/worktree/automation errors before app
  tests could run. No S18/PDF diagnostic was emitted before that blocker, so this
  receipt does not claim a green integrated workspace/app test suite.

S18 is complete at the product-feature level. D13 and A10 remain open for broad
PDF failure-path, helper/package, save-picker and cross-platform acceptance.
