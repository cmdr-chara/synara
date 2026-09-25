# Parity batch 29: reviewed Computer Use drag

## M27 partial: bounded drag action

Computer Use now accepts a typed `drag` action with reviewed window-relative
start/end coordinates and an explicit left/middle/right button.

Execution remains inside the existing one-frame/one-action lease. Synara:
- validates both points against the observed window bounds before execution;
- moves to the reviewed start point;
- revalidates the exact native window before mouse-down, destination move and
  mouse-up;
- sends every xdotool command with the exact reviewed `--window` id;
- consumes the frame through the existing action owner.

A multi-step drag introduces transient button state, so failure handling is
explicit: after a successful mouse-down, any cancellation, stale-window refusal
or xdotool failure triggers a bounded best-effort mouse-up for that same reviewed
window/button before the error is returned. No global pointer fallback, arbitrary
button command or persistent hold action is exposed.

Focused pure regressions cover exact command construction, explicit window
addressing and out-of-bounds destination refusal.

M27 remains open for richer semantic targeting and broader platform behavior.
