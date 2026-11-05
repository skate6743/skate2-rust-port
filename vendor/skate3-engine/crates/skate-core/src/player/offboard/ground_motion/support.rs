//!82D7EEAC..F454 moving support history and800C0 output filtering.
use super::math::*;
use super::{Frame, GroundMotionInput, GroundMotionState};
use crate::trigonometry;

pub(super) fn update(s: &mut GroundMotionState, i: &GroundMotionInput) -> Frame {
    let mut delta = IDENTITY;
    if i.contact_flags_176 & 1 == 0 {
        return delta;
    }
    if s.support_id_352 != i.contact_id_180 {
        if s.support_velocity_removed_715 {
            s.velocity_480 = add(s.velocity_480, s.predicted_support_velocity_272);
            s.speed_704 = length(s.velocity_480);
        }
        s.support_yaw_336 = 0.0;
        s.previous_support_yaw_340 = 0.0;
        s.predicted_support_yaw_344 = 0.0;
        s.support_acceleration_304 = ZERO;
        s.predicted_support_velocity_272 = ZERO;
        s.support_velocity_256 = ZERO;
        s.support_id_352 = i.contact_id_180;
        s.support_velocity_removed_715 = false;
    } else {
        delta = compose(
            inverse_rigid(s.previous_support_frame_192),
            i.contact_frame_32,
        );
        let moved = transform_point(s.frame_0[3], delta);
        let mut velocity = scale(sub(moved, s.frame_0[3]), reciprocal(DT));
        // Host geometric W=0 convention (see math::rotate): transform_point
        // treats the point as homogeneous, ignoring its W and producing W from
        // the frame's W lanes, so this lane is not a velocity component. Left
        // in, it is -W/DT of the frame position, and predicted272 (2v - v')
        // integrated back into frame_0[3] makes any stray W grow by ~-1.618x
        // per tick until Inf - Inf = NaN (issue #10).
        velocity[3] = 0.0;
        let acceleration = madd(
            sub(velocity, s.support_velocity_256),
            reciprocal(DT),
            s.support_acceleration_304,
        );
        s.support_acceleration_304 = scale(acceleration, f32::from_bits(0x3f73_3333));
        s.support_velocity_256 = velocity;
        s.predicted_support_velocity_272 =
            add(velocity, sub(velocity, s.previous_support_velocity_288));
        s.previous_support_velocity_288 = velocity;
        let rotated = transform_vector(s.frame_0[2], delta);
        let flat = sub(rotated, scale(UP, dot(rotated, UP)));
        let direction = normalize_safe(flat, ZERO);
        let sine = clamp_signed(dot(cross(s.frame_0[2], direction), UP));
        let yaw = trigonometry::asin(sine) * RATE;
        s.support_yaw_336 = yaw;
        s.predicted_support_yaw_344 = yaw.mul_add(2.0, -s.previous_support_yaw_340);
        s.previous_support_yaw_340 = yaw;
        if dot(
            s.predicted_support_velocity_272,
            s.predicted_support_velocity_272,
        ) > EPSILON
            && !s.support_velocity_removed_715
        {
            s.velocity_480 = add(s.velocity_480, s.predicted_support_velocity_272.map(|v| -v));
            s.support_velocity_removed_715 = true;
            s.speed_704 = length(s.velocity_480);
        }
    }
    filter(s, i);
    s.previous_support_frame_192 = i.contact_frame_32;
    delta
}

///82D800C0: reference-frame local acceleration, mirrored X/Z, cap then filter.
fn filter(s: &mut GroundMotionState, i: &GroundMotionInput) {
    let f = i.reference_frame_128;
    let columns = [
        [f[0][0], f[1][0], f[2][0], 0.0],
        [f[0][1], f[1][1], f[2][1], 0.0],
        [f[0][2], f[1][2], f[2][2], 0.0],
        ZERO,
    ];
    let mut local = transform_vector(s.support_acceleration_304, columns);
    local[1] = 0.0;
    if i.mirrored_292 {
        local[0] *= -1.0;
        local[2] *= -1.0;
    }
    let mut request = scale(local, f32::from_bits(0x3ccc_cccd));
    let magnitude = length(request);
    if !(magnitude <= 1.0) {
        request = scale(request, 1.0 / magnitude);
    }
    s.filtered_local_acceleration_320 = madd(
        request,
        f32::from_bits(0x3d23_d70a),
        scale(
            s.filtered_local_acceleration_320,
            f32::from_bits(0x3f75_c28f),
        ),
    );
    let speed = length([
        s.support_velocity_256[0],
        0.0,
        s.support_velocity_256[2],
        s.support_velocity_256[0],
    ]) * f32::from_bits(0x3d4c_cccd);
    let nonnegative = select(-speed, 0.0, speed);
    s.support_speed_348 = select(1.0 - nonnegative, nonnegative, 1.0);
}
