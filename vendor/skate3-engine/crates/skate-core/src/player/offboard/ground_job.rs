//! Ground-state PreUpdate82D30D30, prefix copy82D31040 and geometry82D31620.
//! Original Skate3 SHA256:
//!431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
//! Skate2 locomotion82DA0678 uses FeetIK/Skeleton, not this Biped job contract.
use super::{
    contact_toolkit::ContactPrefix,
    controller::{Frame, GroundJob, Vector},
    ground_input::GroundInputOutput,
    ground_query::{ConsumeInput, GroundAdjustment},
};
use crate::math::Vector3;

pub const STEP: f32 = f32::from_bits(0x3c88_8889);

/// The shared toolkit owner publishes BOTH fields, including its reset prefix
/// when refresh completes without a submitted batch. Absence is not readiness.
#[derive(Clone, Copy, Debug)]
pub struct ContactSnapshot {
    pub readiness: i32,
    pub prefix: ContactPrefix,
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub contact: ContactSnapshot,
    pub frame: Frame,
    pub previous_state: u32,
    /// Only processed third-line1056 with its actual valid byte1092 nonzero.
    pub third_line_position: Option<Vector>,
    pub frames_since_teleport: u32,
    pub processed_position: Vector,
    pub processed_velocity: Vector,
    pub controls: GroundInputOutput,
    pub collision_displacements: [Vector; 2],
    pub animation_motion: Vector,
    pub animation_velocity: Vector,
    pub requested_duration: f32,
    pub requested_phase: f32,
    pub override_duration: f32,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub flags_2484: u32,
    pub flags_2488: u32,
}

pub struct Prepared {
    pub job: GroundJob,
    /// Includes state754, retained for the physical state's later consumers even
    /// though the existing controller does not read that byte.
    pub geometry: GroundAdjustment,
}

/// Retained contact192 and timer164 belong to the EXISTING Ground state.
/// The callback consumes the preceding geometry submission exactly once, after
/// contact selection and before controller execution. Errors are never support.
pub fn prepare<E>(
    retained: &mut ContactPrefix,
    timer_164: &mut f32,
    input: Input,
    consume_geometry: impl FnOnce(ConsumeInput) -> Result<GroundAdjustment, E>,
) -> Result<Prepared, E> {
    *timer_164 += STEP;
    // r27 starts at1 (82D30D54); only the ready branch recomputes both-feet.
    let mut both_feet = true;
    if input.contact.readiness > 0 {
        *retained = input.contact.prefix;
        both_feet = retained.flags_176 & 0x30 == 0x30;
    } else if input.previous_state != 501 {
        if let Some(position) = input.third_line_position {
            //82D30DC8..DE4 changes ONLY these three fields.
            retained.flags_176 = 1;
            retained.position = position;
            retained.normal = input.frame[1];
        }
    }
    //82D30DF0..E04: carry/sign sequence is signed <20, including high-bit words.
    let suppress_minimum = (input.frames_since_teleport as i32) < 20;
    if suppress_minimum {
        retained.flags_176 &= !8;
    }
    //82D30EA4 copies before geometry changes ONLY the job's normal at416.
    let copied = *retained;
    let geometry = consume_geometry(ConsumeInput {
        frame_80: query_frame(input.frame),
        contact_position_192: xyz(retained.position),
        contact_flags_368: retained.flags_176,
        reach_364: retained.distance_172,
        previous_input_up_416: xyz(copied.normal),
    })?;
    if !both_feet && !geometry.state_753 && retained.flags_176 & 8 == 0 {
        *timer_164 = 0.;
    }
    // VMX82D30F48: position592 + velocity608 * float1/60, not render/solver dt.
    let animation_position = std::array::from_fn(|i| {
        input.processed_velocity[i].mul_add(STEP, input.processed_position[i])
    });
    let job = GroundJob {
        contact_position: copied.position,
        contact_normal: [
            geometry.input_up_416.x,
            geometry.input_up_416.y,
            geometry.input_up_416.z,
            copied.normal[3],
        ],
        support_frame: copied.support_frame,
        target_position: copied.target_position,
        target_normal: copied.target_normal,
        edge_position: copied.edge_position,
        edge_normal: copied.edge_normal,
        flags: copied.flags_176,
        support_id: copied.support_180,
        collision_displacements: input.collision_displacements,
        animation_motion: input.animation_motion,
        animation_velocity: input.animation_velocity,
        desired_direction: input.controls.state_656,
        animation_position,
        requested_duration: input.requested_duration,
        mirrored: input.flags_2476 & 4 != 0,
        requested_phase: input.requested_phase,
        override_duration: input.override_duration,
        animation_directed: input.flags_2488 & 0x0800_0000 != 0,
        movement: input.controls.state_708,
        steering: input.controls.state_712,
        sprint_pressed: input.flags_2484 & 0x0002_0000 != 0,
        suppress_lean: input.flags_2480 & 0x80 != 0,
        suppress_minimum,
        target_frame_present: geometry.state_752,
        edge_active: geometry.state_753,
        target_frame: native_frame(geometry.frame_768),
        ignore_obstacle: input.flags_2472 & 0x1000_0000 != 0,
    };
    Ok(Prepared { job, geometry })
}

/// Frame-producing portion of Sync82D31E64..82D32070. The same Ground state is
/// updated in place; callers use this frame for Skeleton AND toolkit inputs.
/// Air-launch preparation and board-manager actions retain their shared owners.
pub fn sync_frames(
    state: &mut super::ground_entry::State,
    contact: ContactPrefix,
    result: super::controller::GroundResult,
) -> Frame {
    state.flags_144_to_150[3] = result.sliding;
    state.frame_80 = result.physical_frame;
    if contact.flags_176 & 1 != 0 {
        let delta = std::array::from_fn(|i| contact.position[i] - state.frame_80[3][i]);
        let height = crate::physics::native_arithmetic::dot3(delta, state.frame_80[1]);
        //82D31ECC ble skips the correction; unordered follows the arithmetic.
        if !(height <= 0.) {
            state.frame_80[3] = madd(state.frame_80[1], height, state.frame_80[3]);
        }
    }
    let mut animation = result.animation_frame;
    if state.flags_144_to_150[6] {
        state.duration_180 -= STEP;
        if !(state.duration_180 > 0.) {
            state.duration_180 = 0.;
            state.flags_144_to_150[6] = false;
        }
        state.angle_172 *= f32::from_bits(0x3f66_6666);
        let angle = (-state.angular_velocity_176).mul_add(state.duration_180, state.angle_172);
        let (sin, cos) = crate::trigonometry::sin_cos(angle);
        // Geometric host frame: do not publish Rodrigues permutation scratch W.
        let forward = madd(
            [sin, 0., cos, 0.],
            animation[2][2],
            madd(
                [0., 1., 0., 0.],
                animation[2][1],
                [cos, 0., -sin, 0.].map(|v| v * animation[2][0]),
            ),
        );
        // Original has no zero-length replacement on these two normalizations.
        let right = normalize(cross(animation[1], forward));
        animation[0] = right;
        animation[1] = normalize(cross(forward, right));
        animation[2] = forward;
    }
    animation
}

fn madd(v: Vector, scale: f32, offset: Vector) -> Vector {
    std::array::from_fn(|i| v[i].mul_add(scale, offset[i]))
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn normalize(v: Vector) -> Vector {
    let square = crate::physics::native_arithmetic::dot3(v, v);
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.), inverse);
    }
    v.map(|lane| lane * inverse)
}

fn xyz(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn query_frame(f: Frame) -> super::ground_query::Frame {
    super::ground_query::Frame {
        right: xyz(f[0]),
        up: xyz(f[1]),
        forward: xyz(f[2]),
        position: xyz(f[3]),
    }
}
fn native_frame(f: super::ground_query::Frame) -> Frame {
    [f.right, f.up, f.forward, f.position].map(|v| [v.x, v.y, v.z, 0.])
}
#[cfg(test)]
#[path = "ground_job_tests.rs"]
mod tests;
