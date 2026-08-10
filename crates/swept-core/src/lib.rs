//! Geometry, kinematics and clearance computation for swept path analysis.
//!
//! This crate answers one question: can a given vehicle get through a given
//! opening, and with how much room to spare. It knows nothing about the web,
//! about serialisation, or about wall-clock time — the same inputs always
//! produce the same outputs.
//!
//! # Frame of reference
//!
//! The origin sits at the middle of the opening. `y = 0` is the outer face of
//! the wall, `y > 0` points into the yard, and `x` runs along the road.
//! Lengths are metres, angles are [`units::Radians`].

pub mod clearance;
pub mod curves;
pub mod geometry;
pub mod kinematics;
pub mod scene;
pub mod units;
pub mod vehicle;

/// Crate version, exposed so that callers can report which core produced a
/// result.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_its_version() {
        assert_eq!(version(), "0.1.0");
    }
}
