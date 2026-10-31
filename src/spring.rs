//! Niri-style motion for the slide mode.
//!
//! * `Spring` – closed-form damped spring evaluated at an absolute time `t` since it started,
//!   so the result does not depend on the frame rate (no per-frame lerp).
//!
//! Values mirror niri's `horizontal-view-movement` (damping-ratio 1.0, stiffness 2200).

/// Critically damped: no wobble, no sudden start.
pub const SLIDE_DAMPING_RATIO: f64 = 1.0;
/// Same stiffness as the user's niri `horizontal-view-movement`.
pub const SLIDE_STIFFNESS: f64 = 2200.0;

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub from: f64,
    pub to: f64,
    pub v0: f64,
    /// damping / (2 * mass); mass is 1
    beta: f64,
    omega0: f64,
}

impl Spring {
    pub fn new(from: f64, to: f64, v0: f64, damping_ratio: f64, stiffness: f64) -> Self {
        let omega0 = stiffness.max(0.0).sqrt();
        let beta = damping_ratio.max(0.0) * omega0;
        Self {
            from,
            to,
            v0,
            beta,
            omega0,
        }
    }

    /// Position `t` seconds after the spring started.
    pub fn value_at(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.from;
        }
        if self.omega0 <= 0.0 {
            return self.to;
        }
        let x0 = self.from - self.to;
        let env = (-self.beta * t).exp();
        let diff = self.beta - self.omega0;
        let v = if diff.abs() <= 1e-6 {
            // critically damped
            env * (x0 + (self.beta * x0 + self.v0) * t)
        } else if self.beta < self.omega0 {
            // underdamped
            let w1 = (self.omega0 * self.omega0 - self.beta * self.beta).sqrt();
            env * (x0 * (w1 * t).cos() + ((self.beta * x0 + self.v0) / w1) * (w1 * t).sin())
        } else {
            // overdamped
            let w2 = (self.beta * self.beta - self.omega0 * self.omega0).sqrt();
            env * (x0 * (w2 * t).cosh() + ((self.beta * x0 + self.v0) / w2) * (w2 * t).sinh())
        };
        let out = self.to + v;
        if out.is_finite() { out } else { self.to }
    }

    /// Speed (units per second) `t` seconds after the start.
    pub fn velocity_at(&self, t: f64) -> f64 {
        const H: f64 = 0.001;
        if t <= 0.0 {
            return self.v0;
        }
        (self.value_at(t + H) - self.value_at(t)) / H
    }

    /// True once the spring is visually at rest (sub-pixel and almost no speed).
    pub fn is_settled(&self, t: f64) -> bool {
        if t > 1.5 {
            return true;
        }
        (self.to - self.value_at(t)).abs() < 0.05 && self.velocity_at(t).abs() < 8.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_reaches_target_without_overshoot() {
        let s = Spring::new(0.0, 500.0, 0.0, SLIDE_DAMPING_RATIO, SLIDE_STIFFNESS);
        let mut prev = 0.0;
        for i in 0..400 {
            let v = s.value_at(i as f64 / 1000.0);
            assert!(
                v >= prev - 1e-9 && v <= 500.0 + 1e-6,
                "monotonic, no overshoot"
            );
            prev = v;
        }
        assert!(s.is_settled(0.5));
        assert!(!s.is_settled(0.01));
    }

    #[test]
    fn spring_starts_at_rest() {
        let s = Spring::new(100.0, 0.0, 0.0, 1.0, 2200.0);
        assert_eq!(s.value_at(0.0), 100.0);
        assert!(s.velocity_at(0.0).abs() < 1e-9);
    }
}
