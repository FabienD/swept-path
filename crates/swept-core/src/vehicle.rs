//! The vehicle: its dimensions, and the outline used for collision checks.
//!
//! Coordinates are local to the vehicle, with the origin on the rear axle and
//! `x` pointing forward. The rear bumper therefore sits at `-rear_overhang`
//! and the front bumper at `wheelbase + front_overhang`.

use crate::geometry::Point;

/// Why a set of vehicle dimensions was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleError {
    /// A dimension that must be strictly positive was not. Carries the field
    /// name so the caller can point at it; the caller owns the wording shown
    /// to a user.
    NonPositive(&'static str),
    /// The front overhang leaves no room for a rear overhang, meaning the
    /// wheelbase and front overhang already exceed the total length.
    FrontOverhangTooLarge,
    /// Mirrors cannot be narrower than the body they fold against.
    MirrorsNarrowerThanBody,
    /// A published turning radius that no vehicle of this wheelbase could
    /// hold. Almost always a transcription error, or a figure that is not the
    /// radius it claims to be.
    ImplausibleTurningRadius,
}

/// Number of stations sampled along the body, from rear bumper to front.
///
/// ARBITRARY — carried over from the prototype (`index.html:278`), which walks
/// five stations. Denser sampling costs time in the inner loop of every
/// search; five was never justified by measurement.
pub const BODY_STATIONS: usize = 5;

/// A vehicle, validated at construction.
///
/// ```
/// use swept_core::vehicle::Vehicle;
///
/// // Lexus LBX
/// let v = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).unwrap();
/// assert!((v.rear_overhang - 0.760).abs() < 1e-9);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vehicle {
    /// Distance between axles, in metres.
    pub wheelbase: f64,
    /// From the front axle to the front bumper, in metres.
    pub front_overhang: f64,
    /// From the rear axle to the rear bumper, in metres. Derived.
    pub rear_overhang: f64,
    /// Body width, mirrors excluded, in metres.
    pub width: f64,
    /// Width over the mirrors, in metres. This is almost always the critical
    /// dimension.
    pub mirror_width: f64,
    /// Height of the lowest point of the bodywork, wheels excluded, in metres.
    ///
    /// What decides whether the vehicle can overhang a kerb rather than be
    /// stopped by it. Manufacturers publish this figure, and three caveats
    /// come with it: it is quoted **unladen** — a loaded vehicle settles two
    /// to four centimetres — the measuring convention varies between makers,
    /// and **deformable parts are usually excluded**, though the front bumper
    /// lip is precisely what overhangs first when the vehicle turns.
    ///
    /// The bias therefore runs both ways: conservative on the flank, where a
    /// figure taken under the whole vehicle underestimates the sill, and
    /// possibly optimistic on the nose.
    pub ground_clearance: f64,
    /// Radius traced by the **rear axle centre** at full lock, in metres.
    ///
    /// This is the pivot the bicycle model turns about, and it is *not* the
    /// figure manufacturers publish. A kerb-to-kerb radius is traced by the
    /// outer front wheel and is markedly larger — for a Lexus LBX, 5.20 m
    /// published against 3.59 m here. Feeding the published number in makes
    /// the vehicle turn about half again as wide as it can, and the simulator
    /// invents manoeuvres to compensate.
    ///
    /// Use [`pivot_radius_from_kerb`] to convert.
    pub min_turning_radius: f64,
}

/// Converts a kerb-to-kerb radius into the rear-axle pivot radius.
///
/// Manufacturers publish the circle traced by the outer front wheel. The rear
/// axle centre runs inside it: subtract half the track to reach the front
/// wheel's own axle line, then take the leg of the right triangle whose
/// hypotenuse that is and whose other leg is the wheelbase.
///
/// ```
/// use swept_core::vehicle::pivot_radius_from_kerb;
///
/// // Lexus LBX: 5.20 m published, 2.58 m wheelbase, 1.56 m track.
/// let pivot = pivot_radius_from_kerb(5.20, 2.58, 1.56).unwrap();
/// assert!((pivot - 3.59).abs() < 0.02);
/// ```
///
/// # Errors
///
/// Returns [`VehicleError::ImplausibleTurningRadius`] when the figures cannot
/// describe a real vehicle — a radius smaller than the wheelbase it is
/// supposed to swing, for instance.
pub fn pivot_radius_from_kerb(
    kerb_radius: f64,
    wheelbase: f64,
    track: f64,
) -> Result<f64, VehicleError> {
    for value in [kerb_radius, wheelbase, track] {
        if !value.is_finite() || value <= 0.0 {
            return Err(VehicleError::ImplausibleTurningRadius);
        }
    }
    let at_front_axle = kerb_radius - track / 2.0;
    let squared = at_front_axle.mul_add(at_front_axle, -(wheelbase * wheelbase));
    if squared <= 0.0 {
        return Err(VehicleError::ImplausibleTurningRadius);
    }
    Ok(squared.sqrt())
}

impl Vehicle {
    /// Builds a vehicle, deriving the rear overhang from the total length.
    ///
    /// # Errors
    ///
    /// Returns [`VehicleError`] when a dimension is not strictly positive, when
    /// the front overhang leaves no rear overhang, or when the mirrors are
    /// narrower than the body.
    pub fn new(
        wheelbase: f64,
        length: f64,
        front_overhang: f64,
        width: f64,
        mirror_width: f64,
        ground_clearance: f64,
        min_turning_radius: f64,
    ) -> Result<Self, VehicleError> {
        for (value, name) in [
            (wheelbase, "wheelbase"),
            (length, "length"),
            (front_overhang, "front_overhang"),
            (width, "width"),
            (mirror_width, "mirror_width"),
            (ground_clearance, "ground_clearance"),
            (min_turning_radius, "min_turning_radius"),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(VehicleError::NonPositive(name));
            }
        }

        let rear_overhang = length - wheelbase - front_overhang;
        if rear_overhang <= 0.0 {
            return Err(VehicleError::FrontOverhangTooLarge);
        }
        if mirror_width < width {
            return Err(VehicleError::MirrorsNarrowerThanBody);
        }

        Ok(Self {
            wheelbase,
            front_overhang,
            rear_overhang,
            width,
            mirror_width,
            ground_clearance,
            min_turning_radius,
        })
    }

    /// The four contact patches, in the vehicle's own frame.
    ///
    /// At the corners of a `width × wheelbase` rectangle — the **body** width,
    /// not the track. The track separates the wheels' median planes, so the
    /// outer edge of a tyre sits half a tyre width further out: on the LBX,
    /// 1.56 m of track and 225 mm tyres put that edge 0.893 m from the axis
    /// against 0.913 m for half the body, because wheel arches follow tyres.
    /// Using half the body width therefore costs about two centimetres and
    /// spares the caller two measurements nobody has to hand — where the track
    /// alone would have cost thirteen.
    ///
    /// The two centimetres are not guaranteed to fall on the safe side: wide
    /// rims or a different offset can bring a tyre flush with the arch.
    #[must_use]
    pub fn wheels(&self) -> [Point; 4] {
        let half = self.width / 2.0;
        [
            Point::new(0.0, half),
            Point::new(0.0, -half),
            Point::new(self.wheelbase, half),
            Point::new(self.wheelbase, -half),
        ]
    }

    /// The points sampled along the vehicle outline, in local coordinates.
    ///
    /// Five stations along each flank, the two bumper centres, and the two
    /// mirrors level with the front axle. The mirrors matter more than the
    /// rest: they are almost always the first thing to touch a pillar.
    #[must_use]
    pub fn envelope(&self) -> Vec<Point> {
        let half_body = self.width / 2.0;
        let half_mirrors = self.mirror_width / 2.0;
        let rear = -self.rear_overhang;
        let front = self.wheelbase + self.front_overhang;

        let mut points = Vec::with_capacity(2 * BODY_STATIONS + 4);
        for i in 0..BODY_STATIONS {
            #[allow(clippy::cast_precision_loss)]
            let fraction = i as f64 / (BODY_STATIONS - 1) as f64;
            let x = rear + (front - rear) * fraction;
            points.push(Point::new(x, half_body));
            points.push(Point::new(x, -half_body));
        }
        points.push(Point::new(rear, 0.0));
        points.push(Point::new(front, 0.0));
        points.push(Point::new(self.wheelbase, half_mirrors));
        points.push(Point::new(self.wheelbase, -half_mirrors));
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    /// Lexus LBX, the prototype's default vehicle.
    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    #[test]
    fn the_wheels_sit_at_the_corners_of_the_body() {
        let wheels = lbx().wheels();
        let half = 1.825 / 2.0;
        // Rear axle at the origin, front axle a wheelbase ahead, both at the
        // body's own half-width — see `wheels` for why the body and not the
        // track.
        assert!((wheels[0].x - 0.0).abs() < EPS);
        assert!((wheels[0].y - half).abs() < EPS);
        assert!((wheels[1].x - 0.0).abs() < EPS);
        assert!((wheels[1].y + half).abs() < EPS);
        assert!((wheels[2].x - 2.580).abs() < EPS);
        assert!((wheels[2].y - half).abs() < EPS);
        assert!((wheels[3].x - 2.580).abs() < EPS);
        assert!((wheels[3].y + half).abs() < EPS);
    }

    #[test]
    fn rejects_a_ground_clearance_of_zero() {
        // A vehicle flat on the ground overhangs nothing, which would silently
        // turn every kerb back into a wall — the very state this batch leaves.
        let error = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.0, 5.2)
            .expect_err("zero is not a ground clearance");
        assert_eq!(error, VehicleError::NonPositive("ground_clearance"));
    }

    #[test]
    fn derives_the_rear_overhang() {
        // 4.190 − 2.580 − 0.850 = 0.760
        assert!((lbx().rear_overhang - 0.760).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_front_overhang_that_leaves_no_rear() {
        let err = Vehicle::new(2.580, 4.190, 1.700, 1.825, 2.029, 0.18, 5.2).unwrap_err();
        assert_eq!(err, VehicleError::FrontOverhangTooLarge);
    }

    #[test]
    fn rejects_mirrors_narrower_than_the_body() {
        let err = Vehicle::new(2.580, 4.190, 0.850, 1.825, 1.700, 0.18, 5.2).unwrap_err();
        assert_eq!(err, VehicleError::MirrorsNarrowerThanBody);
    }

    #[test]
    fn rejects_non_positive_dimensions() {
        assert_eq!(
            Vehicle::new(0.0, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).unwrap_err(),
            VehicleError::NonPositive("wheelbase")
        );
        assert_eq!(
            Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, -1.0).unwrap_err(),
            VehicleError::NonPositive("min_turning_radius")
        );
    }

    #[test]
    fn converts_a_kerb_to_kerb_radius_into_a_pivot_radius() {
        // Lexus LBX: 5.2 m kerb to kerb, 2.58 m wheelbase, 1.56 m track. The
        // rear axle centre runs on a much tighter circle than the outer front
        // wheel that traces the published figure.
        let pivot = pivot_radius_from_kerb(5.2, 2.58, 1.56).expect("plausible");
        assert!((pivot - 3.59).abs() < 0.02, "got {pivot}");
    }

    #[test]
    fn a_pivot_radius_is_always_tighter_than_the_published_one() {
        for kerb in [4.8_f64, 5.2, 5.7, 6.2] {
            let pivot = pivot_radius_from_kerb(kerb, 2.58, 1.56).expect("plausible");
            assert!(pivot < kerb, "{pivot} should be under {kerb}");
        }
    }

    #[test]
    fn rejects_a_radius_too_small_for_the_wheelbase() {
        // A car cannot turn inside its own wheelbase; such a figure is a
        // transcription error, not a very agile vehicle.
        assert_eq!(
            pivot_radius_from_kerb(2.0, 2.58, 1.56),
            Err(VehicleError::ImplausibleTurningRadius)
        );
    }

    #[test]
    fn samples_the_envelope_including_the_mirrors() {
        let v = lbx();
        let envelope = v.envelope();

        // Five stations on each side, plus front and rear centres, plus the
        // two mirrors: fourteen points.
        assert_eq!(envelope.len(), 14);

        // The widest points are the mirrors, level with the front axle.
        let widest = envelope.iter().map(|p| p.y.abs()).fold(0.0_f64, f64::max);
        assert!((widest - v.mirror_width / 2.0).abs() < EPS);

        // The envelope spans from the rear bumper to the front bumper.
        let rearmost = envelope.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let frontmost = envelope.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        assert!((rearmost + v.rear_overhang).abs() < EPS);
        assert!((frontmost - (v.wheelbase + v.front_overhang)).abs() < EPS);
    }
}
