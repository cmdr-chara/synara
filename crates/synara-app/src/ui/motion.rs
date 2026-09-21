//! Native timing for Synara's presentation contract, independent of web widgets.
use std::time::{Duration, Instant};

pub const DRAWER_DURATION: Duration = Duration::from_millis(300);
pub const PANE_DURATION: Duration = Duration::from_millis(140);
pub const MESSAGE_DURATION: Duration = Duration::from_millis(180);

pub fn pane_duration() -> Duration { PANE_DURATION.mul_f32(super::motion_multiplier()) }
pub fn message_duration() -> Duration { MESSAGE_DURATION.mul_f32(super::motion_multiplier()).max(Duration::from_millis(1)) }

/// CSS cubic-bezier evaluates y at the parameter whose x is elapsed time.
/// Treating elapsed time as the parameter produces a different animation.
pub fn cubic_bezier(time: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if time <= 0.0 {
        return 0.0;
    }
    if time >= 1.0 {
        return 1.0;
    }
    let coordinate = |t: f32, a: f32, b: f32| {
        let inverse = 1.0 - t;
        3.0 * inverse * inverse * t * a + 3.0 * inverse * t * t * b + t * t * t
    };
    let (mut lower, mut upper) = (0.0, 1.0);
    for _ in 0..20 {
        let t = (lower + upper) * 0.5;
        if coordinate(t, x1, x2) < time {
            lower = t;
        } else {
            upper = t;
        }
    }
    coordinate((lower + upper) * 0.5, y1, y2)
}

pub fn drawer_easing(time: f32) -> f32 {
    cubic_bezier(time, 0.32, 0.72, 0.0, 1.0)
}
pub fn ease_out(time: f32) -> f32 {
    cubic_bezier(time, 0.0, 0.0, 0.58, 1.0)
}

pub struct Drawer {
    from: f32,
    target: f32,
    started: Instant,
    duration: Duration,
}

impl Drawer {
    pub fn new(open: bool) -> Self {
        let target = if open { 1.0 } else { 0.0 };
        Self {
            from: target,
            target,
            started: Instant::now(),
            duration: Duration::ZERO,
        }
    }

    pub fn target_open(&self) -> bool {
        self.target > 0.0
    }

    pub fn set_open(&mut self, open: bool, now: Instant, reduced_motion: bool) {
        let from = self.value(now);
        self.target = if open { 1.0 } else { 0.0 };
        self.from = from;
        self.started = now;
        self.duration = if reduced_motion {
            Duration::ZERO
        } else {
            DRAWER_DURATION.mul_f32((self.target - from).abs() * super::motion_multiplier())
        };
    }

    pub fn value(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return self.target;
        }
        let time =
            now.saturating_duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32();
        self.from + (self.target - self.from) * drawer_easing(time)
    }

    pub fn running(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started) < self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawer_has_the_reference_css_curve() {
        // Independent parametric point: t=0.5 gives x=0.245, y=0.77.
        assert!((drawer_easing(0.245) - 0.77).abs() < 0.00001);
        assert_eq!(drawer_easing(0.0), 0.0);
        assert_eq!(drawer_easing(1.0), 1.0);
    }

    #[test]
    fn reversal_is_continuous_and_reduced_motion_settles_immediately() {
        let now = Instant::now();
        let mut drawer = Drawer::new(true);
        assert!(!drawer.running(now));
        drawer.set_open(false, now, false);
        let midway = now + Duration::from_millis(90);
        let visible = drawer.value(midway);
        assert!(visible > 0.0 && visible < 1.0);
        drawer.set_open(true, midway, false);
        assert!((drawer.value(midway) - visible).abs() < f32::EPSILON);
        assert_eq!(drawer.value(midway + DRAWER_DURATION), 1.0);
        drawer.set_open(false, midway, true);
        assert_eq!(drawer.value(midway), 0.0);
        assert!(!drawer.running(midway));
    }
}
