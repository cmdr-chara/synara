# Windows terminal backend acceptance

The backend completion sweep replaces the old direct-write Windows terminal with
worker-owned ConPTY I/O and the same bounded TerminalScreen parser used on POSIX.
No product UI files are changed.

## Implemented contract

- NativeTerminal exposes the same text/key/paste/resize/scrollback/render snapshot
  and shutdown methods on Windows and POSIX.
- Input is bounded by 256 messages and 1 MiB including the in-flight write. A full
  queue returns Limit rather than waiting for the child to read stdin.
- Keyboard and clipboard encoding respects current application-cursor and
  bracketed-paste modes. Clipboard approval is checked before queuing.
- Raw history, grid dimensions, scrollback, VT control strings and terminal replies
  use the shared bounded parser. A pipe write never holds the screen mutex.
- Failed startup owns and stops any already-spawned child. Cancellation is an
  independent flag, not another message that can be trapped in a full queue.
- Shutdown waits at most three seconds and returns Timeout when the native helper
  cannot finish. Completion reports worker cleanup failures explicitly. This is
  not a claim that every native ClosePseudoConsole call can be interrupted.

## Evidence scope

`crates/synara-runtime/tests/terminal_windows.rs` launches only its own disposable
Rust test executable in real ConPTY. It covers Unicode output and historical
rendering, a child that does not consume input, an output flood with resizing,
invalid dimensions and failed process startup. The fixture entry is intentionally
ignored by the normal harness and is invoked by its owning parent tests.

State: OPEN pending native Windows CI for this checkpoint.

The existing taskkill fallback does not prove race-free Windows process-tree
containment. Job Object assignment before execution, descendant ownership after
leader exit, and adversarial detached descendants remain OPEN under M1/M4. These
are implementation gaps, not external blockers. Linux process-group/session
acceptance must not be used as Windows evidence.
