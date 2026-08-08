//! Units used across the domain.
//!
//! Lengths are plain `f64` in metres. Angles get a dedicated type because the
//! user interface works in degrees while every computation here works in
//! radians, and that is the one unit confusion this domain actually suffers
//! from.

use std::ops::{Add, Neg, Sub};

/// An angle, in radians.
///
/// ```
/// use swept_core::units::Radians;
///
/// let right = Radians::from_degrees(90.0);
/// assert!((right.get() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Radians(f64);

impl Radians {
    /// Builds an angle from a value already expressed in radians.
    #[must_use]
    pub const fn new(radians: f64) -> Self {
        Self(radians)
    }

    /// Builds an angle from a value expressed in degrees.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Self {
        Self(degrees.to_radians())
    }

    /// The value in radians.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The value in degrees. Display only — no computation in this crate
    /// should need it.
    #[must_use]
    pub fn to_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Sine and cosine in a single call, which is how this type is almost
    /// always consumed.
    #[must_use]
    pub fn sin_cos(self) -> (f64, f64) {
        self.0.sin_cos()
    }
}

impl Add for Radians {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Radians {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Neg for Radians {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    const EPS: f64 = 1e-12;

    #[test]
    fn converts_degrees_to_radians() {
        assert!((Radians::from_degrees(180.0).get() - PI).abs() < EPS);
        assert!((Radians::from_degrees(90.0).get() - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn round_trips_through_degrees() {
        assert!((Radians::from_degrees(37.5).to_degrees() - 37.5).abs() < EPS);
    }

    #[test]
    fn adds_subtracts_and_negates() {
        let a = Radians::from_degrees(90.0);
        let b = Radians::from_degrees(30.0);
        assert!(((a + b).to_degrees() - 120.0).abs() < EPS);
        assert!(((a - b).to_degrees() - 60.0).abs() < EPS);
        assert!(((-b).to_degrees() + 30.0).abs() < EPS);
    }

    #[test]
    fn yields_sine_and_cosine_together() {
        let (sin, cos) = Radians::from_degrees(90.0).sin_cos();
        assert!((sin - 1.0).abs() < EPS);
        assert!(cos.abs() < EPS);
    }
}
