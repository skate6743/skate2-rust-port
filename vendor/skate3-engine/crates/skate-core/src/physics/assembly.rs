//! Board constraint construction from physical body snapshots.
//!
//! The default TU3 assembly has six joints, two truck drives and a hook drive. Wheel drives
//! are an alternate configuration, disabled in the recovered stock assembly.
//! World collision and gameplay force production do not belong in this module.
use super::{
    board::{BODY_COUNT, BodyId},
    constraint_batch::compile_active,
    drive_frames::{RetailAffineTransform, RetailDriveFrames},
    drive_parameters::RetailDriveDynamics,
    drive_solver::{RetailDriveBodyState, RetailDriveRows, build_drive_rows},
    joint_builder::{RetailJointBodyInput, RetailJointBuildInput, build_retail_joint_jacobian},
    joint_records::default_joint_records,
    rigid_body::{RetailBodyRates, RetailInertiaDynamics, pack_world_inverse_inertia},
    solver::JointConstraint,
    truck_frames::steering_drive_frames,
};

#[derive(Clone, Copy, Debug)]
pub struct BodySnapshot {
    pub state_flags: u32,
    pub rates: RetailBodyRates,
    pub inertia: RetailInertiaDynamics,
}

/// Separate animated target body registered by CreateHookDrive 82C0D330.
/// It is not one of the seven physical board parts and must not be substituted
/// for the deck. Startup supplies its native body state explicitly.
#[derive(Clone, Debug)]
pub struct BoardHook {
    pub body: BodySnapshot,
    pub drive: super::hook_drive::HookDriveState,
}

pub const HOOK_REACTION: usize = BODY_COUNT;

/// The caller owns the snapshots, force accumulators and tick timing. No pose
/// is adjusted here to force the board onto a plane or into an animation.
pub struct BoardConstraints {
    /// Eligible rows only; retained in native registration order.
    pub joints: Vec<JointConstraint>,
    /// Front truck, back truck, then hook; native creation/insertion order.
    pub drives: Vec<RetailDriveRows>,
}

impl BoardConstraints {
    pub fn build(
        bodies: &[BodySnapshot; BODY_COUNT],
        hook: &BoardHook,
        frames: [RetailDriveFrames; 3],
        truck_dynamics: RetailDriveDynamics,
        delta_seconds: f32,
    ) -> Self {
        let joints = compile_active(
            default_joint_records(),
            |record| {
                [
                    bodies[record.live_body_a().index()].state_flags,
                    bodies[record.live_body_b().index()].state_flags,
                ]
            },
            |record| {
                let a = record.live_body_a();
                let b = record.live_body_b();
                JointConstraint {
                    jacobian: build_retail_joint_jacobian(RetailJointBuildInput {
                        parameters: record.parameters,
                        frames: record.frames,
                        body_a: joint_body(bodies[a.index()]),
                        body_b: joint_body(bodies[b.index()]),
                        time_step: delta_seconds,
                        // Guest pointer metadata is not dereferenced by the host solver.
                        joint_guest_address: 0,
                    }),
                    reaction_a: a.index(),
                    reaction_b: b.index(),
                }
            },
        );
        let mut drives = compile_active(
            0..2,
            |&index| {
                let truck = [BodyId::FrontTruck, BodyId::BackTruck][index];
                [
                    bodies[truck.index()].state_flags,
                    bodies[BodyId::Deck.index()].state_flags,
                ]
            },
            |index| {
                let truck = [BodyId::FrontTruck, BodyId::BackTruck][index];
                build_drive_rows(
                    drive_body(bodies[truck.index()], truck.index()),
                    drive_body(bodies[BodyId::Deck.index()], BodyId::Deck.index()),
                    frames[index],
                    truck_dynamics,
                    delta_seconds,
                )
            },
        );
        drives.extend(compile_active(
            [()],
            |_| {
                [
                    hook.body.state_flags,
                    bodies[BodyId::Deck.index()].state_flags,
                ]
            },
            |_| {
                build_drive_rows(
                    drive_body(hook.body, HOOK_REACTION),
                    drive_body(bodies[BodyId::Deck.index()], BodyId::Deck.index()),
                    frames[2],
                    hook.drive.solver_dynamics(),
                    delta_seconds,
                )
            },
        ));
        Self { joints, drives }
    }
}

/// SetTruckDriveFrames 82C0B9C0 followed by the active-drive quaternion
/// preparation in Island::Step_Solver2 82763B08. Hook normalization is a live
/// state write; it is not deferred to the animation frame setters.
pub fn prepare_drive_frames(
    base: [RetailAffineTransform; 2],
    targets: [f32; 2],
    hook: &mut BoardHook,
) -> [RetailDriveFrames; 3] {
    use super::drive_preparation::normalize_drive_frames;
    let trucks = steering_drive_frames(base, targets).map(|frames| {
        let mut raw = [0u32; 16];
        for (i, frame) in [frames.body_a, frames.body_b].into_iter().enumerate() {
            let q = frame.orientation;
            let t = frame.translation;
            raw[i * 8..i * 8 + 8]
                .copy_from_slice(&[q.x, q.y, q.z, q.w, t.x, t.y, t.z, 0.0].map(f32::to_bits));
        }
        normalize_drive_frames(&mut raw);
        hook_frames(&raw)
    });
    normalize_drive_frames(&mut hook.drive.frames);
    [trucks[0], trucks[1], hook_frames(&hook.drive.frames)]
}

fn hook_frames(words: &[u32; 16]) -> super::drive_frames::RetailDriveFrames {
    use super::{
        drive_frames::{RetailDriveFrame, RetailDriveFrames},
        rigid_body::RetailQuaternion,
    };
    use crate::math::Vector3;
    let f = |i| f32::from_bits(words[i]);
    let frame = |offset: usize| RetailDriveFrame {
        orientation: RetailQuaternion {
            x: f(offset),
            y: f(offset + 1),
            z: f(offset + 2),
            w: f(offset + 3),
        },
        translation: Vector3::new(f(offset + 4), f(offset + 5), f(offset + 6)),
    };
    RetailDriveFrames {
        body_a: frame(0),
        body_b: frame(8),
    }
}

fn joint_body(body: BodySnapshot) -> RetailJointBodyInput {
    let rates = body.rates;
    RetailJointBodyInput {
        reaction_guest_address: 0,
        state: body.state_flags,
        orientation: rates.orientation,
        center_of_mass: rates.position,
        basis: rates.basis,
        linear_velocity: rates.linear_velocity,
        angular_velocity: rates.angular_velocity,
        force_acceleration: rates.force_acceleration,
        torque_acceleration: rates.torque_acceleration,
        inverse_mass: body.inertia.inverse_mass,
        world_inverse_inertia: pack_world_inverse_inertia(rates.world_inverse_inertia),
    }
}

fn drive_body(body: BodySnapshot, reaction_index: usize) -> RetailDriveBodyState {
    let rates = body.rates;
    RetailDriveBodyState {
        reaction_index,
        state: body.state_flags,
        orientation: rates.orientation,
        basis: rates.basis,
        center_of_mass: rates.position,
        linear_velocity: rates.linear_velocity,
        angular_velocity: rates.angular_velocity,
        force_acceleration: rates.force_acceleration,
        torque_acceleration: rates.torque_acceleration,
        inverse_mass: body.inertia.inverse_mass,
        world_inverse_inertia: pack_world_inverse_inertia(rates.world_inverse_inertia),
    }
}

#[cfg(test)]
#[path = "tests/assembly.rs"]
mod tests;
