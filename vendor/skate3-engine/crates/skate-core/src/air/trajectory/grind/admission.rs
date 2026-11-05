//! Original S3 82D6A840 admission and82D6AF58 landing publication.
use super::*;
///position/material stay intact; time, frame, trajectory and normal change.
pub fn apply_admitted_target(
    prediction: &mut Prediction,
    candidate: GrindTrajectoryCandidate,
    correction: Vector,
    support_normal: Vector,
    reference_velocity: Vector,
    maximum_adjust: f32,
    landing_velocity_scalar: f32,
    landing_max_angle: f32,
) {
    super::super::launch::adjust_trajectory(
        &mut prediction.request.trajectory,
        candidate.frame,
        correction,
        maximum_adjust,
    );
    prediction.result.contact_frame = candidate.frame;
    prediction.result.contact_time = candidate.frame as f32 * STEP;
    prediction.result.contact_normal = landing_normal(
        candidate.direction,
        reference_velocity,
        support_normal,
        landing_velocity_scalar,
        landing_max_angle,
    );
}

///82D6AF58: bias by cross-rail velocity, bound against the investigated
///support normal, reject the rail axis, then normalize.
pub fn landing_normal(
    direction: Vector,
    velocity: Vector,
    support: Vector,
    velocity_scalar: f32,
    maximum_angle_degrees: f32,
) -> Vector {
    let across = sub(velocity, scale(direction, dot(direction, velocity)));
    let target = madd(across, -velocity_scalar, UP);
    let limited = clamp_angle(
        target,
        support,
        maximum_angle_degrees * f32::from_bits(0x3c8e_fa35),
    );
    //82C1E170 removes only the positive normalized-direction component,
    //multiplying by the ORIGINAL supplied direction.
    let component = dot(limited, normalize(direction));
    normalize(if component > 0.0 {
        sub(limited, scale(direction, component))
    } else {
        limited
    })
}

///82BD3E78, shared with the offboard surface-frame port.
fn clamp_angle(target: Vector, reference: Vector, limit: f32) -> Vector {
    let a = normalize(target);
    let b = normalize(reference);
    let axis = cross(a, b);
    let angle = crate::trigonometry::acos(dot(a, b).clamp(-1.0, 1.0));
    let turns = angle.mul_add(f32::from_bits(0x3e22_f983), 0.5).floor();
    let wrapped = (-turns).mul_add(f32::from_bits(0x40c9_0fdb), angle);
    if wrapped.abs() < limit || f32::from_bits(0x3780_0000) > dot(axis, axis) {
        return target;
    }
    let unit = normalize(axis);
    let (sin, cos) = crate::trigonometry::sin_cos(-limit * 0.5);
    let q = scale(unit, sin);
    scale(
        madd(
            cross(q, madd(reference, cos, cross(q, reference))),
            2.0,
            reference,
        ),
        length(target),
    )
}

///Admission and displacement portion82D6A840. The caller retries the next
///ranked candidate on None, then applies82D609E0 and82D6AF58 on success.
///This deliberately requires native surface evidence before any correction.
pub fn admitted_displacement(
    prediction: Prediction,
    candidate: GrindTrajectoryCandidate,
    edge: Primitive,
    reference_velocity: Vector,
    processed_592: Vector,
    surface: GrindSurfaceEvidence,
    limits: &GrindAssistLimits,
    effective_lock_distance: &mut Option<f32>,
) -> Option<Vector> {
    let natural_landing = prediction
        .request
        .trajectory
        .position_at(prediction.result.contact_time);
    if candidate.point[1] - natural_landing[1] < f32::from_bits(0xbecc_cccd)
        || candidate.direction[1] > 0.9
    {
        return None;
    }
    let direction = normalize(sub(edge.end, edge.start));
    let perpendicular = sub(
        reference_velocity,
        scale(direction, dot(direction, reference_velocity)),
    );
    if (perpendicular[1] * (1.0 - direction[1].abs())).abs() > limits.max_downward_speed {
        return None;
    }
    if surface.kind == 3 {
        return None;
    }
    let mut horizontal = perpendicular;
    horizontal[1] = 0.0;
    let max_speed = if surface.kind == 2 {
        limits.max_speed_squared_ledge
    } else {
        limits.max_speed_squared_rail
    };
    if dot(horizontal, horizontal) > max_speed {
        return None;
    }
    let delta = sub(candidate.trajectory_point, candidate.point);
    let lock_distance = if surface.kind == 2 {
        let incoming = dot(delta, surface.side) > 0.0;
        let body = dot(sub(processed_592, edge.start), surface.side) > 0.0;
        limits.lock_distance
            * limits.ledge_scalars[if incoming { 0 } else { 2 } + if body { 0 } else { 1 }]
    } else {
        limits.lock_distance
    };
    let lock_distance = lock_distance.max(0.1);
    *effective_lock_distance = Some(lock_distance);
    if candidate.distance >= lock_distance {
        return None;
    }
    let miss = sub(candidate.trajectory_point, edge.start);
    let mut correction = scale(
        sub(
            miss,
            scale(candidate.direction, dot(candidate.direction, miss)),
        ),
        -1.0,
    );
    let distance = length(correction);
    if distance >= limits.lock_distance {
        let half_dimension =
            limits.deck_dimensions[1].mul_add(0.5, limits.deck_dimensions[0] * 0.5);
        correction = scale(
            correction,
            (-limits.tip_scalar).mul_add(half_dimension, distance) / distance,
        );
    }
    let original = prediction.request.trajectory.velocity;
    let adjusted = madd(correction, reciprocal(candidate.time), original);
    let angle = if dot(original, original) * dot(adjusted, adjusted) > f32::from_bits(0x38d1_b717) {
        angle_between(original, adjusted)
    } else {
        0.0
    };
    if angle > limits.maximum_adjust_angle * f32::from_bits(0x3c8e_fa35) {
        return None;
    }
    Some(correction)
}
