//! Geometry actually observed by Ground::ManageHangUps82D39510.
//! The caller reads only query completion, geometry kind104 and flag29 from
//! 82C20C08; unrelated grind surface classifications are not used in this path.
use super::{
    board_motion_output::{dot, inverse_length_squared},
    board_world::BoardWorld,
};
use crate::math::Vector3;

#[derive(Clone, Copy, Debug)]
pub struct HangGeometryInput {
    /// Processed1312,1328 and1232, respectively.
    pub edge_start: Vector3,
    pub edge_end: Vector3,
    pub reference_point: Vector3,
}
#[derive(Clone, Copy, Debug)]
pub struct HangLine {
    pub start: Vector3,
    pub end: Vector3,
    pub radius: f32,
}

/// Full six-line branch82C20728 used by82D397A0. Input's optional seventh
/// line byte is zeroed by82C20530 and never set by this ground caller.
pub fn hang_lines(input: HangGeometryInput, deck_center_to_truck: f32) -> Option<[HangLine; 6]> {
    let delta = sub(input.edge_end, input.edge_start);
    let first = cross(Vector3::new(0., 1., 0.), delta);
    let raw_up = cross(delta, first);
    let (up, length) = normalize_or_retain(raw_up);
    let (direction, _) = normalize_or_retain(delta);
    if length < f32::from_bits(0x3727_c5ac) {
        return None;
    }
    let on_edge = madd(
        direction,
        dot(sub(input.reference_point, input.edge_start), direction),
        input.edge_start,
    );
    // Preserve the native pair of subtractions rather than canceling them.
    let center = sub(input.reference_point, sub(input.reference_point, on_edge));
    let side = cross(up, direction);
    let short_up = scale(up, 0.04);
    let near_side = scale(side, 0.09);
    let far_side = scale(side, deck_center_to_truck);
    let far_up = scale(up, deck_center_to_truck * f32::from_bits(0x3f87_ae14));
    let last_up = scale(up, 0.09);
    let line = |center: Vector3, up: Vector3, radius| HangLine {
        start: add(center, up),
        end: sub(center, up),
        radius,
    };
    Some([
        line(add(center, near_side), short_up, 0.),
        line(sub(center, near_side), short_up, 0.),
        line(add(center, far_side), far_up, 0.),
        line(sub(center, far_side), far_up, 0.),
        line(center, short_up, 0.),
        line(add(center, last_up), near_side, 0.001),
    ])
}

/// 82C20ED4..82C20F84 and Ground's82D397EC..82D39810. We execute all six
/// native queries, then use the exact two classification fields read by Ground.
pub fn detect_hung_geometry(
    world: &BoardWorld,
    input: HangGeometryInput,
    deck_center_to_truck: f32,
) -> Result<bool, &'static str> {
    let Some(lines) = hang_lines(input, deck_center_to_truck) else {
        return Ok(false);
    };
    let mut fractions = [None; 6];
    for (i, line) in lines.into_iter().enumerate() {
        fractions[i] = world
            .query_swept_line(line.start, line.end, line.radius)?
            .map(|hit| hit.geometry.fraction);
    }
    Ok(hung_classification(fractions))
}
fn hung_classification(hits: [Option<f32>; 6]) -> bool {
    if (hits[0].is_some() && hits[1].is_some()) || hits[4].is_some() {
        return false;
    }
    let kind = if hits[0].is_some() {
        if hits[2].is_some_and(|fraction| fraction < 0.65) {
            2
        } else {
            1
        }
    } else if hits[1].is_some() {
        if hits[3].is_some_and(|fraction| fraction < 0.65) {
            2
        } else {
            1
        }
    } else {
        0
    };
    kind != 2
}
fn normalize_or_retain(v: Vector3) -> (Vector3, f32) {
    let square = dot(v, v);
    let inverse = inverse_length_squared(square, 2);
    let length = if square == 0. { 0. } else { square * inverse };
    (
        if length > f32::from_bits(0x3586_37bd) {
            scale(v, inverse)
        } else {
            v
        },
        length,
    )
}
fn cross(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(
        (-a.z).mul_add(b.y, a.y * b.z),
        (-a.x).mul_add(b.z, a.z * b.x),
        (-a.y).mul_add(b.x, a.x * b.y),
    )
}
fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(a: Vector3, k: f32) -> Vector3 {
    Vector3::new(a.x * k, a.y * k, a.z * k)
}
fn madd(a: Vector3, k: f32, b: Vector3) -> Vector3 {
    Vector3::new(
        a.x.mul_add(k, b.x),
        a.y.mul_add(k, b.y),
        a.z.mul_add(k, b.z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hanging_geometry_distinguishes_rail_side_and_blocked_deck() {
        let input = HangGeometryInput {
            edge_start: Vector3::ZERO,
            edge_end: Vector3::new(0., 0., 2.),
            reference_point: Vector3::new(4., 5., 1.),
        };
        let lines = hang_lines(input, 0.243).unwrap();
        assert_eq!(lines[0].start, Vector3::new(0.09, 0.04, 1.));
        assert_eq!(lines[0].end, Vector3::new(0.09, -0.04, 1.));
        assert_eq!(lines[1].start.x, -0.09);
        assert_eq!(lines[5].radius, 0.001);
        assert_eq!(lines[5].start, Vector3::new(0.09, 0.09, 1.));
        assert_eq!(lines[5].end, Vector3::new(-0.09, 0.09, 1.));
        assert!(hung_classification([
            Some(0.5),
            None,
            None,
            None,
            None,
            None
        ]));
        assert!(!hung_classification([
            Some(0.5),
            None,
            Some(0.64),
            None,
            None,
            None
        ]));
        assert!(!hung_classification([
            Some(0.5),
            Some(0.5),
            None,
            None,
            None,
            None
        ]));
        assert!(!hung_classification([
            None,
            None,
            None,
            None,
            Some(0.5),
            None
        ]));
    }
}
