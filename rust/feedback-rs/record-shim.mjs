// Record shim for the feedback corpus.
//
// `buildFeedbackSubmission` falls back to `new Date().toISOString()` when the
// input carries no `now`, which would make every recorded case volatile. The
// `Date` constructor is frozen here so the corpus is deterministic; the Rust
// port replays the frozen instant via SYNARA_FEEDBACK_NOW_MS.
const FIXED = 946684800000 // 2000-01-01T00:00:00.000Z

class FixedDate extends Date {
  constructor(...args) {
    return args.length === 0 ? super(FIXED) : super(...args)
  }

  static now() {
    return FIXED
  }
}
globalThis.Date = FixedDate

export { formatFeedbackSummary, buildFeedbackSubmission } from '../../apps/web/src/feedback.ts'
