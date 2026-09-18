# Windows process-tree ownership checkpoint

Roadmap owner: M1/M4.

Generic native subprocesses on Windows now acquire a safe Win32 Job Object through
`win32job 2.0.3`. The job is configured with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and the spawned child is assigned before
Synara publishes the process handle to callers. The Job handle remains owned by
the runtime supervisor until supervision ends.

If Job creation/configuration/assignment fails, the just-spawned child receives an
immediate kill request and the launch fails instead of proceeding without the
claimed ownership boundary.

Existing `taskkill /T` remains the graceful/best-effort stop path. The Job Object
is the final ownership guarantee when supervision ends, including normal parent
exit, cancellation, last-owner drop and supervisor-task teardown. No unsafe code is
added to Synara; the workspace-wide `unsafe_code = "forbid"` policy remains
unchanged.

The native Windows regression re-executes only the isolated Rust test binary. Its
parent fixture starts a sleeping descendant and then exits normally. The test waits
for the parent result and verifies the descendant disappears afterward. This case
would not be established merely by sending `taskkill /T` while the parent is
still alive.

This checkpoint strengthens generic Windows subprocess ownership. M1 remains open
for complete ConPTY Job Object integration and actual macOS process/PTY lifecycle
acceptance. It is not evidence for those surfaces.
