//! Original S3 82D60C80/82D60B98/82D6A398/82D6A168.
use super::*;

///82C1F310 after provider traversal; pairwise rejection preserves global order.
pub fn trajectory_box_filter(
    indices: &[usize],
    primitives: &[Primitive],
    takeoff: Vector,
) -> Vec<usize> {
    let directions: Vec<_> = indices
        .iter()
        .map(|&i| {
            let d = sub(primitives[i].end, primitives[i].start);
            scale(d, inverse_length(dot(d, d)))
        })
        .collect();
    let mut accepted = Vec::with_capacity(indices.len());
    for (i, &index) in indices.iter().enumerate() {
        let a = primitives[index];
        let mut keep = true;
        for (j, &other) in indices.iter().enumerate() {
            if i == j {
                continue;
            }
            let crossed = cross(directions[i], directions[j]);
            if !(dot(crossed, crossed) < f32::from_bits(0x38d1_b717)) {
                continue;
            }
            let b = primitives[other];
            let delta = sub(b.start, a.start);
            let perpendicular = sub(delta, scale(directions[j], dot(directions[j], delta)));
            let distance = dot(perpendicular, perpendicular);
            if !(distance > f32::from_bits(0x38d1_b717) && distance < f32::from_bits(0x3c23_d70b)) {
                continue;
            }
            let middle = scale(add(a.start, a.end), 0.5);
            let from = sub(b.start, middle);
            if !(dot(from, sub(b.end, middle)) < 0.) {
                continue;
            }
            let up = normalize(cross(cross(directions[i], UP), directions[i]));
            let height = dot(delta, up);
            let alternative = add(
                middle,
                sub(from, scale(directions[j], dot(directions[j], from))),
            );
            let first = sub(middle, takeoff);
            let second = sub(alternative, takeoff);
            if (height > -0.02 && dot(first, first) > dot(second, second)) || height > 0.02 {
                keep = false;
            }
        }
        if keep {
            accepted.push(index);
        }
    }
    accepted
}

///82D60C80's quadratic roots followed by82D60B98's later-root selection.
/// A double root must be strictly positive. The two-root branch takes max;
/// it does not add a positive-time restriction absent from the source.
pub fn descending_plane_time(t: Trajectory, point: Vector, normal: Vector) -> Option<f32> {
    let c = dot(normal, t.position) - dot(point, normal);
    let b = dot(normal, t.velocity);
    let a = dot(normal, scale(t.acceleration, reciprocal(2.0)));
    let discriminant = b * b - (4.0 * a) * c;
    if discriminant < 0.0 {
        return None;
    }
    let inverse = reciprocal(2.0 * a);
    if discriminant > 0.0 {
        let root = discriminant * inverse_length(discriminant);
        Some((inverse * (-b + root)).max(inverse * (-b - root)))
    } else {
        let time = inverse * -b;
        (time > 0.0).then_some(time)
    }
}

///82D6A398. The padding is physics_trajectory+516, not a host snap radius.
pub fn consider_grind_primitive(
    prediction: Prediction,
    edge: Primitive,
    primitive: usize,
    padding: f32,
) -> Option<GrindTrajectoryCandidate> {
    let mut delta = sub(edge.end, edge.start);
    let normal_unscaled = cross(delta, cross(UP, delta));
    let normal = scale(
        normal_unscaled,
        inverse_length(dot(normal_unscaled, normal_unscaled)),
    );
    let rail_length = length(delta);
    //830BD320 is initialized to1 by82F825D0. The original endpoints remain
    //unchanged; only the direction and endpoint overrun vector are capped.
    if rail_length > 1.0 {
        delta = scale(delta, reciprocal(rail_length));
    }
    let offset = madd(normal, prediction.request.radius, scale(normal, padding));
    let time = descending_plane_time(
        prediction.request.trajectory,
        add(edge.start, offset),
        normal,
    )?;
    let trajectory_point = sub(prediction.request.trajectory.position_at(time), offset);
    let direction = normalize(delta);
    let difference = sub(trajectory_point, edge.start);
    let distance = length(cross(difference, direction));
    let point = madd(direction, dot(direction, difference), edge.start);
    let extension = scale(delta, 0.1);
    //Strict endpoint test82D6A708. No closest-point clamping.
    if !(dot(
        sub(point, sub(edge.start, extension)),
        sub(point, add(edge.end, extension)),
    ) < 0.0)
    {
        return None;
    }
    let velocity = prediction.request.trajectory.velocity;
    let approach = normalize(sub(velocity, scale(normal, dot(normal, velocity))));
    let mut horizontal = sub(point, prediction.request.trajectory.position);
    horizontal[1] = 0.0;
    let angle = folded_approach_angle(horizontal, direction, normal);
    Some(GrindTrajectoryCandidate {
        point,
        trajectory_point,
        direction,
        approach,
        distance,
        time,
        angle,
        frame: (time * f32::from_bits(0x426f_ffff)) as i32,
        primitive,
    })
}

///8296EC98's oriented angle then82E09C80's actual fractional-turn fold.
///Keep the wrap arithmetic: acos(abs(dot)) is not binary32 equivalent.
fn folded_approach_angle(a: Vector, b: Vector, normal: Vector) -> f32 {
    let mut angle = angle_between(a, b);
    let aa = dot(a, a);
    let bb = dot(b, b);
    if aa > f32::from_bits(0x38d1_b717) && bb > f32::from_bits(0x38d1_b717) {
        let unit = |v, square| {
            let r = crate::physics::native_arithmetic::reciprocal_square_root_estimate(square);
            scale(v, (r * 0.5).mul_add((-square).mul_add(r * r, 1.0), r))
        };
        if dot(cross(unit(a, aa), unit(b, bb)), normal) < 0.0 {
            angle = f32::from_bits(0x40c9_0fdb) - angle;
        }
    }
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let wrapped = (fraction - if fraction > 0.5 { 1.0 } else { 0.0 }) * f32::from_bits(0x40c9_0fdb);
    let sign = if wrapped > 0.0 { 1.0 } else { -1.0 };
    let magnitude = wrapped * sign;
    let folded = if magnitude > f32::from_bits(0x3fc9_0fdb) {
        magnitude - std::f32::consts::PI
    } else {
        magnitude
    };
    (sign * folded).abs()
}

///82D6A168 consumes one winner per call. Retrying after a rejected target
///must retain the remaining insertion order, not sort the list by distance.
pub fn take_best_grind(
    candidates: &mut Vec<GrindTrajectoryCandidate>,
    difficulty_distance: f32,
    height_penalty: &PointGraph<8>,
) -> Option<GrindTrajectoryCandidate> {
    let mut chosen = None;
    let mut fallback = 1000.0;
    let mut best_angle = std::f32::consts::PI;
    let mut height = -10000.0;
    for (i, candidate) in candidates.iter().enumerate() {
        if candidate.distance < difficulty_distance {
            let benefit = best_angle - candidate.angle;
            if benefit > height_penalty.evaluate(candidate.point[1] - height) {
                height = candidate.point[1];
                fallback = -1.0;
                best_angle = candidate.angle;
                chosen = Some(i);
            }
        } else if candidate.distance < fallback {
            fallback = candidate.distance;
            chosen = Some(i);
        }
    }
    chosen.map(|i| candidates.remove(i))
}
