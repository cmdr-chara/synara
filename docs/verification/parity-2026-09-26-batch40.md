# Parity continuation verification - batch 40

Date: 2026-09-26

## Delivered

### M25 Richer model/context controls

The direct-model review already exposed reviewed Fast/Balanced/Thinking effort
presets only when the selected model advertises matching reasoning levels, output
budgets bounded by model metadata, custom history windows and keyboard model
cycling.

This batch adds the remaining explicit context-compaction workflow. **Fit context**
derives the largest recent complete-turn history window that fits beneath the
reviewed model context limit using a conservative UTF-8/base64 byte upper bound,
while reserving requested output plus space for the next prompt. The action edits
only the review draft and still requires route confirmation followed by explicit
Send. It never deletes, rewrites or summarizes the durable transcript.

The estimator includes retained user/assistant text and stored image payloads,
never expands beyond 256 reviewed turns, and returns current-message-only when
even the newest prior turn exceeds the safe budget.

## Inventory effect

- M25 complete: major remaining 10 -> 9.
- Shipped feature slices 86 -> 87.
- Execution total 12 -> 11.
