//! Original TU3 trajectory launch calculations82D67D08/82D67320/82D682E8.
use super::{LaunchInfo, QueryRequest, SelectorInput, SelectorSettings, Trajectory, math::*};

pub(super) fn adjust_velocity(
    info: &mut LaunchInfo,
    input: SelectorInput,
    s: &SelectorSettings,
) -> bool {
    let n = input.ground_normal;
    let v = info.start_velocity;
    let mut normal = scale(n, dot(v, n));
    normal[1] = 0.0;
    let combined = add([0.0, v[1], 0.0, 0.0], normal);
    let residual = sub(v, combined);
    let lean = (input.directional_input - 0.25).max(-1.0).min(1.0);
    let aligned = normalize(sub(UP, scale(n, lean * s.vert_jump_align_max_angle)));
    if n[1] >= s.vert_jump_align_max_ground_normal_y {
        return false;
    }
    if normalize(combined)[1] > s.vert_jump_align_min_direction_y {
        //The second term really uses UNIT velocity, not the original speed.
        let left = scale(
            add(scale(aligned, length(combined)), residual),
            s.vert_jump_align_factor,
        );
        let right = scale(normalize(v), 1.0 - s.vert_jump_align_factor);
        info.start_velocity = scale(normalize(add(left, right)), length(v));
        true
    } else {
        if !info.player_jumped && input.previous_physics_state != 200 {
            let normal = scale(n, dot(v, n));
            info.start_velocity = add(
                scale(normal, s.natural_air_off_verts_scalar),
                sub(v, normal),
            );
        }
        false
    }
}

pub(super) fn candidate_velocities(info: LaunchInfo, s: &SelectorSettings) -> Vec<Vector> {
    let count = usize::from(info.trajectory_count.min(7));
    let mut velocities = Vec::with_capacity(count);
    if count == 0 {
        return velocities;
    }
    let v = info.start_velocity;
    velocities.push(v);
    let right = normalize(cross(UP, v));
    let forward = normalize(cross(right, UP));
    let speed = length(v);
    let cone_speed = if info.player_jumped {
        length(info.com_velocity)
    } else {
        speed
    };
    let radians = f32::from_bits(0x3c8e_fa35);
    let x = info.cone_angle_x * radians;
    let z = (s.cone_angle_z_vs_speed.evaluate(cone_speed * 0.05) * info.cone_angle_z) * radians;
    let cone_speed = cone_speed.max(s.speed_factor_min).min(s.speed_factor_max);
    let right = scale(right, crate::trigonometry::sin(x) * cone_speed);
    let forward = scale(forward, crate::trigonometry::sin(z) * cone_speed);
    if count > 1 {
        let step = f32::from_bits(0x40c9_0fdb) / (count - 1) as f32;
        let mut angle = 0.0;
        for _ in 1..count {
            let candidate = madd(
                forward,
                crate::trigonometry::cos(angle),
                madd(right, crate::trigonometry::sin(angle), v),
            );
            velocities.push(scale(normalize(candidate), length(candidate).min(speed)));
            angle += step;
        }
    }
    velocities
}

#[derive(Clone, Debug)]
pub(super) struct LaunchBatch {
    pub requests: Vec<QueryRequest>,
    pub velocities: Vec<Vector>,
    pub origin: Vector,               //2864
    pub board_position: Vector,       //2848
    pub local_board_position: Vector, //2832
    pub local_com_position: Vector,   //2784
    pub com_displacement: Vector,     //2768
}
pub(super) fn batch(info: LaunchInfo, input: SelectorInput, s: &SelectorSettings) -> LaunchBatch {
    let velocities = candidate_velocities(info, s);
    let ground = s
        .displacement_vs_ground_normal
        .evaluate(input.ground_normal[1].abs());
    let speed = s
        .displacement_vs_speed
        .evaluate(input.board_vertical_velocity * 0.1);
    let blend = speed.max(ground);
    let height = dot(
        sub(info.animation_com_position, input.contact_position),
        info.reckoning_transform[1],
    ) - s.trajectory_radius;
    let displacement =
        height.min(s.trajectory_displacement) * (1.0 - blend) + s.trajectory_displacement * blend;
    let mut origin = sub(
        info.animation_com_position,
        scale(info.reckoning_transform[1], displacement),
    );
    let mut board_position = info.board_position;
    if info.use_position_override {
        origin = info.start_position_override;
        board_position = info.board_position_override;
    }
    let com_displacement = sub(info.animation_com_position, origin);
    let duration = max_time(info.start_velocity, s);
    let requests = velocities
        .iter()
        .map(|&velocity| QueryRequest {
            trajectory: Trajectory {
                position: madd(velocity, info.timestep, origin),
                velocity,
                acceleration: input.gravity,
                duration,
            },
            radius: s.trajectory_radius,
            start_error: s.trajectory_error_start,
            end_error: s.trajectory_error_end,
        })
        .collect();
    LaunchBatch {
        requests,
        velocities,
        origin,
        board_position,
        com_displacement,
        local_board_position: transform(info.reckoning_inverse, sub(info.board_position, origin)),
        local_com_position: transform(info.reckoning_inverse, com_displacement),
    }
}
///82D68698 deliberately uses the original -19.6 and -1/9.8 coefficients.
fn max_time(velocity: Vector, s: &SelectorSettings) -> f32 {
    let square = velocity[1].mul_add(
        velocity[1],
        -(s.trajectory_max_drop * f32::from_bits(0xc19c_cccd)),
    );
    let root = if square == 0.0 {
        0.0
    } else {
        square * inverse_length(square)
    };
    ((-velocity[1] - root) * f32::from_bits(0xbdd0_fac6)).min(s.trajectory_max_time)
}

///tTrajectory::AdjustTrajectory82D609E0 changes velocity only.
pub(super) fn adjust_trajectory(
    trajectory: &mut Trajectory,
    frame: i32,
    adjustment: Vector,
    maximum: f32,
) {
    if frame <= 0 {
        return;
    }
    let correction = scale(adjustment, reciprocal(frame as f32 * STEP));
    let scalar = if dot(correction, correction) > maximum * maximum {
        maximum / length(correction)
    } else {
        1.0
    };
    trajectory.velocity = madd(correction, scalar, trajectory.velocity);
}
