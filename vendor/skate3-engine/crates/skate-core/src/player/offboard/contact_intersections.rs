//! Normal constraints82D82940/82E0A258 and obstacle intersections82D82AF0.
use super::{
    contact_queries::{Input, V},
    contact_records::{Direction, Record, Records, Source},
    contact_segments::length_inverse,
};
use crate::physics::native_arithmetic::dot3;
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn unit(a: V) -> V {
    let (length, inverse) = length_inverse(a);
    if length > f32::from_bits(0x3586_37bd) {
        a.map(|x| x * inverse)
    } else {
        [0.; 4]
    }
}
fn negate(a: V) -> V {
    a.map(|x| f32::from_bits(x.to_bits() ^ 0x80000000))
}
pub fn clamp_normal(a: V, b: V, normal: &mut V) -> bool {
    let a = unit(a);
    let b = unit(b);
    let axis = unit(cross(a, b));
    let dot = dot3(axis, *normal);
    let projected = unit(std::array::from_fn(|i| normal[i] - axis[i] * dot));
    let product = ((dot3(axis, axis) * dot3(projected, projected)) * dot3(a, a)) * dot3(b, b);
    if !(product >= f32::from_bits(0x3dcc_cccd)) {
        return false;
    }
    *normal = if dot3(cross(a, projected), axis) > 0. && dot3(cross(projected, b), axis) > 0. {
        projected
    } else if dot3(projected, a) > dot3(projected, b) {
        a
    } else {
        b
    };
    true
}
///82E0A060 accepts strictly opposite sides; endpoint-on-plane is not a hit.
pub fn plane_segment(origin: V, normal: V, start: V, end: V) -> Option<V> {
    let a = dot3(normal, sub(start, origin));
    let b = dot3(normal, sub(end, origin));
    if a * b >= 0. {
        return None;
    }
    let total = b.abs() + a.abs();
    let wa = b.abs() / total;
    let wb = a.abs() / total;
    Some(std::array::from_fn(|i| start[i].mul_add(wa, end[i] * wb)))
}
pub fn constrain(input: Input, records: &mut Records) {
    let mut previous = input.position;
    for record in &mut records.surface {
        let height = dot3(sub(record.position, previous), input.surface_up);
        if !(height.abs() >= f32::from_bits(0x3a83_126f)) {
            record.normal = input.surface_up;
        } else {
            let forward = if height > 0. {
                negate(input.surface_forward)
            } else {
                input.surface_forward
            };
            clamp_normal(forward, input.surface_up, &mut record.normal);
        }
        //82940 loads this record's position into the next iteration's base.
        previous = record.position;
    }
    let count = records.surface.len();
    for i in 1..count {
        let a = records.surface[i - 1];
        let b = records.surface[i];
        for j in 0..records.obstacle.len() {
            let obstacle = records.obstacle[j];
            if obstacle.flags & 3 != 0
                || a.coordinates[0] > obstacle.coordinates[0]
                || obstacle.coordinates[0] > b.coordinates[0]
            {
                continue;
            }
            insert_intersections(input, records, a, b, j);
        }
    }
    super::contact_sort::sort(&mut records.surface);
}
fn insert_intersections(input: Input, records: &mut Records, a: Record, b: Record, index: usize) {
    let obstacle = records.obstacle[index];
    let delta = sub(b.position, a.position);
    let close = f32::from_bits(0x3c23_d70a);
    if dot3(a.normal, b.normal) > f32::from_bits(0x3f7d_70a4)
        && dot3(a.normal, obstacle.normal) > f32::from_bits(0x3f7d_70a4)
        && close > dot3(delta, b.normal).abs()
    {
        let mut delta = sub(obstacle.position, a.position);
        if close > length_inverse(delta).0 {
            delta = sub(obstacle.position, b.position);
        }
        if close > dot3(delta, b.normal).abs() {
            return;
        }
    }
    let mut normal = obstacle.normal;
    let forward = if b.coordinates[1] > a.coordinates[1] {
        negate(input.surface_forward)
    } else {
        input.surface_forward
    };
    clamp_normal(forward, input.surface_up, &mut normal);
    let first_dot = dot3(a.normal, delta);
    let second_dot = dot3(b.normal, delta);
    let extent = f32::from_bits(0x3f8c_cccd);
    let extended_first = std::array::from_fn(|i| {
        (delta[i] - a.normal[i] * first_dot).mul_add(extent, a.position[i])
    });
    let extended_second =
        std::array::from_fn(|i| b.position[i] - (delta[i] - b.normal[i] * second_dot) * extent);
    let hits = [
        plane_segment(obstacle.position, normal, a.position, extended_first),
        plane_segment(obstacle.position, normal, extended_second, b.position),
    ];
    let low = if b.coordinates[1] > a.coordinates[1] {
        a.coordinates[1]
    } else {
        b.coordinates[1]
    };
    let high = if a.coordinates[1] > b.coordinates[1] {
        a.coordinates[1]
    } else {
        b.coordinates[1]
    };
    let length = length_inverse(delta).0;
    for point in hits.into_iter().flatten() {
        let d = sub(point, input.position);
        let forward = dot3(input.surface_forward, d);
        let height = dot3(input.surface_up, d);
        if !(forward >= a.coordinates[0]
            && b.coordinates[0] >= forward
            && height >= low
            && high >= height)
        {
            continue;
        }
        let fraction = length_inverse(sub(point, a.position)).0 / length;
        let distance = fraction.mul_add(b.distance - a.distance, a.distance);
        let mut n = normal;
        records.insert(
            input,
            point,
            &mut n,
            Source::Support,
            Direction::Reverse,
            distance,
        );
        records.obstacle[index].flags |= 4;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_is_projected_then_constrained_to_the_native_wedge() {
        let a = [0., 0., -1., 0.];
        let b = [0., 1., 0., 0.];
        let mut n = [1., 1., -1., 0.];
        assert!(clamp_normal(a, b, &mut n));
        assert_eq!(n[0], 0.);
        assert!(n[1] > 0.7 && n[2] < -0.7);
        let mut outside = [0., -1., 0., 0.];
        clamp_normal(a, b, &mut outside);
        assert_eq!(outside, a);
    }
    #[test]
    fn plane_intersection_rejects_endpoint_contacts() {
        assert_eq!(
            plane_segment(
                [0.; 4],
                [0., 1., 0., 0.],
                [0., -1., 0., 0.],
                [0., 1., 0., 0.]
            ),
            Some([0.; 4])
        );
        assert!(plane_segment([0.; 4], [0., 1., 0., 0.], [0.; 4], [0., 1., 0., 0.]).is_none());
    }
}
