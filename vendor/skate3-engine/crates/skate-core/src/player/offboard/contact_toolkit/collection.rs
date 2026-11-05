//! TU3 82D81610: authored-edge/secondary-line merge, then primary lines.
use super::analyzer_math::{madd, plane_segment};
use super::{Batch, ProbeLayout, QueryResults, Samples, Vector, dot, scale, sub};

pub(super) fn collect(batch: &Batch, layout: &ProbeLayout, results: &QueryResults) -> Samples {
    let input = batch.input;
    let mut samples = Samples::default();
    let main = results.trajectories[0];
    if main.valid() {
        samples.insert(input, main.contact_position, main.landing_normal, 1, -1., 0);
    }
    let mut edge_points: Vec<Option<Vector>> = vec![None; batch.secondary.len()];
    for &[start, end] in &results.edges {
        let delta = sub(end, start);
        if folded_angle(delta, input.forward).abs() > 30. * 0.017453292 {
            if let Some(point) = plane_segment(input.position, input.right, start, end) {
                samples.insert(input, point, input.up, 2, -1., 0);
            }
            continue;
        }
        let a = dot(sub(start, input.position), input.forward);
        let b = dot(sub(end, input.position), input.forward);
        let at_origin = madd(delta, a / (a - b), start);
        if dot(input.right, sub(at_origin, input.position)).abs() > 0.5 {
            continue;
        }
        for ((local, world), point) in layout
            .secondary
            .iter()
            .zip(&batch.secondary)
            .zip(&mut edge_points)
        {
            let z = local.line.start[2];
            if z < a.min(b) || z > a.max(b) {
                continue;
            }
            let on_edge = madd(delta, (z - a) / (b - a), start);
            let projected = madd(
                input.up,
                dot(input.up, sub(on_edge, world.line.start)),
                world.line.start,
            );
            if dot(input.right, sub(projected, on_edge)).abs() >= 0.05 {
                continue;
            }
            if point.is_none_or(|old| projected[1] > old[1]) {
                *point = Some(projected);
            }
        }
    }
    for (descriptor, edge) in batch.secondary.iter().zip(edge_points) {
        let hit = results.lines[descriptor.forward_index];
        match (hit, edge) {
            (Some(hit), Some(edge)) if edge[1] > hit.position[1] => {
                samples.insert(input, edge, input.up, 1, -1., 0);
            }
            (Some(hit), _) => {
                samples.insert(input, hit.position, hit.normal, 1, -1., 0);
            }
            (None, Some(edge)) => {
                samples.insert(input, edge, input.up, 1, -1., 0);
            }
            (None, None) => {}
        }
    }
    for descriptor in &batch.primary {
        for index in [Some(descriptor.forward_index), descriptor.reverse_index]
            .into_iter()
            .flatten()
        {
            if let Some(hit) = results.lines[index] {
                samples.insert(input, hit.position, hit.normal, 0, -1., 0);
            }
        }
    }
    super::profile::sort(&mut samples.ground);
    super::profile::sort(&mut samples.other);
    samples
}

/// 8296EBB0 followed by 82E09C80. One inverse-sqrt refinement per input.
fn folded_angle(a: Vector, b: Vector) -> f32 {
    let aa = dot(a, a);
    let bb = dot(b, b);
    let mut angle = 0.;
    if aa > f32::from_bits(0x38d1b717) && bb > f32::from_bits(0x38d1b717) {
        let unit = |v, square| {
            let r = crate::physics::reciprocal_sqrt::estimate(square);
            scale(v, (r * 0.5).mul_add((-square).mul_add(r * r, 1.), r))
        };
        angle = crate::trigonometry::acos(dot(unit(a, aa), unit(b, bb)).clamp(-1., 1.));
    }
    let turns = angle * f32::from_bits(0x3e22f983);
    let fraction = turns - turns.floor();
    let signed = (fraction - if fraction > 0.5 { 1. } else { 0. }) * f32::from_bits(0x40c90fdb);
    let sign = if signed <= 0. { -1. } else { 1. };
    let magnitude = signed * sign;
    sign * if magnitude > 1.5707964 {
        magnitude - 3.1415927
    } else {
        magnitude
    }
}
