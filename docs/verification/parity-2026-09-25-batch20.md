# Parity batch 20: focused editor diff review

## M21 partial: changes-only comparison view

The native editor comparison panel can now switch between the complete bounded
comparison and a Changes only review. Filtering removes only unchanged rows from
presentation. Changed and informational rows retain their original indices and
old/new line numbers from the full diff.

Restore block and Copy block still receive the original diff index and therefore
retain the existing task/project/root/path/tab owner check, comparison generation
check, exact expected-line check, and exact current-diff recomputation before any
buffer mutation or clipboard action.

The filter does not write the file, mutate the unsaved editor buffer, alter the
comparison reference, or touch the Git index. Restore all remains separately
confirmation-gated.

Focused unit coverage verifies that filtered rows map back to the exact original
diff indices, retain Added/Removed/Info rows, and that the unfiltered mode returns
the complete index sequence.

M21 remains open for deeper interactive hunk editing and any staging workflow that
can safely preserve the user's existing Git index.


## M21 partial: explicit change-block review cursor

The comparison panel now has a presentation-only change cursor. Previous/Next
wrap across the starts of actual contiguous added/removed blocks, never context
or informational rows. The selected block is highlighted and its position is
shown in the comparison summary.

Toolbar Copy selected block and Restore selected block reuse the existing block
operations rather than bypassing them. Restore therefore still rechecks the
comparison owner, generation, exact expected line, exact recomputed current diff
and editor composition/save state before changing only the unsaved buffer.

Any editor-buffer change, new comparison request or replacement comparison
snapshot clears the cursor, preventing a selection from being carried into a
different diff. No Git index or staging operation was added.

Focused pure tests cover block-start discovery and forward/backward wrap behavior.
M21 remains open for deeper hunk editing and an index-safe staging contract.
