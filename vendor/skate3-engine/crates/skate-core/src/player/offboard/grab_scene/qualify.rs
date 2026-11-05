//! S3 CanGrabSpline82E08DB8/82E08EE8 and actual polyline callees.
//! S2 82E4B4F8/82E4A9A8 corroborate projection, angles and box, but lack
//! S3's horizontal-facing/approach gates. No endpoints-only spline substitute.
mod math;
mod polyline;
mod selection;
#[cfg(test)]
mod tests;
use super::Record;
use crate::player::offboard::ground_sync::{BoardLimits, Bounds};
pub use math::closest_point;
use math::*;
pub use selection::best_spline;

pub fn qualify(record: &Record, position: Vector, bounds: Bounds, limits: BoardLimits) -> bool {
    let length = record.scalar(176);
    let mut margin = limits.margin;
    if margin > length * 0.5 {
        margin = length * 0.5;
    }
    let distance = polyline::nearest_distance(record, position)
        .max(margin)
        .min(length - margin);
    let point = polyline::at_distance(record, distance);
    let endpoints = record.endpoints();
    let direction = sub(endpoints[0], endpoints[1]);
    let forward = bounds.frame[2];
    let facing = dot(normalize(flatten(forward)), normalize(flatten(direction))).abs();
    let side = normalize(cross([0., 1., 0., 0.], direction));
    let approach = record.vector(96);
    let approach_side = dot(side, approach);
    //82E0915C/16C scalar skip branches, followed by ordered angular test.
    if approach_side.abs() <= 0.1 || dot(forward, side) * approach_side >= 0. || !(facing < 0.8) {
        return false;
    }
    let reach = flatten(sub(point, position));
    if !(dot(reach, reach) > 0.) {
        return false;
    }
    if !(limits.angle_a > angle(reach, approach.map(|v| -v))) {
        return false;
    }
    //82E09228..2A0 chooses a real neighboring point along the full polyline.
    let mut step = length * 0.5;
    if step > f32::from_bits(0x3c23d70a) {
        step = f32::from_bits(0x3c23d70a);
    }
    let next_distance = if distance + step > length {
        distance - step
    } else {
        distance + step
    };
    let tangent = sub(polyline::at_distance(record, next_distance), point);
    let horizontal = flatten(tangent);
    if !(dot(horizontal, horizontal) > f32::from_bits(0x37800000)) {
        return false;
    }
    let turns = angle(tangent, horizontal) * f32::from_bits(0x3e22f983);
    let fraction = turns - turns.floor();
    let mut slope =
        ((fraction - if fraction > 0.5 { 1. } else { 0. }) * f32::from_bits(0x40c90fdb)).abs();
    if slope > f32::from_bits(0x3fc90fdb) {
        slope = f32::from_bits(0x40490fdb) - slope;
    }
    if !(limits.angle_b > slope) {
        return false;
    }
    //82E093E8 rigid inverse followed by inclusive bounds on all XYZ axes.
    let local = inverse_point(bounds.frame, point);
    (0..3).all(|i| local[i] >= -bounds.extents[i] && local[i] <= bounds.extents[i])
}
