//! Persistent board physics for the host game.
//!
//! Ownership and spawning are host policy. Physical initialization follows TU3
//! AddRigidBody 82AE5928 and Reset 82776140: dynamic bodies start at rest with
//! gravity accumulated, while the separate hook has zero inverse mass/tensor.
//! Part pose conversion reuses 82BD4318/82C0B2C8; no packed body is retained
//! beside the state consumed and updated by BoardStep.
use crate::math::{Basis3, Vector3};

use super::{
    assembly::{BoardHook, BodySnapshot},
    board::{BODY_COUNT, BodyId},
    board_pose::{PartPose, PoseMatrix, part_transform, set_board_transform, set_part_transform},
    board_step::{AttachedStep, BoardCollision, BoardStep, BoardStepSettings},
    drive_frames::RetailAffineTransform,
    force_queue::BoardForceQueue,
    hook_drive::HookDriveState,
    rigid_body::{
        RetailBodyMassProperties, RetailBodyRates, RetailInertiaDynamics, RetailLocalMassFrame,
        RetailQuaternion, RetailSimulationStep, world_inverse_inertia,
    },
};

/// The host explicitly chooses the board's initial physical mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardMotion {
    Active,
    Frozen,
    Static,
}

impl BoardMotion {
    fn flags(self) -> u32 {
        match self {
            Self::Active => 4,
            Self::Frozen => 2,
            Self::Static => 1,
        }
    }
}

pub struct BoardRuntime {
    bodies: [BodySnapshot; BODY_COUNT],
    mass_frames: [RetailLocalMassFrame; BODY_COUNT],
    hook: BoardHook,
    step: BoardStep,
    forces: BoardForceQueue,
    ///Original assembly/part group: standard82C090C0=4, wipeout82C09160=7.
    collision_group: u32,
}

impl BoardRuntime {
    /// Opt-in inspection of completed rows; capture never changes solver inputs.
    pub fn take_solver_diagnostics(&mut self, enabled: bool) -> Option<String> {
        self.step.diagnostic_capture = enabled;
        self.step.diagnostic_snapshot.take()
    }

    /// `authored` contains part poses in board space. `spawn` places the deck
    /// part in world space, and moves the other parts by the same rigid delta.
    /// All mass properties come from the caller's physical data producer.
    pub fn new(
        masses: [RetailBodyMassProperties; BODY_COUNT],
        authored: [RetailAffineTransform; BODY_COUNT],
        spawn: RetailAffineTransform,
        simulation: RetailSimulationStep,
        mode: BoardMotion,
    ) -> Self {
        let mut parts = core::array::from_fn(|i| {
            let mut part = PartPose {
                transform: pose_words(authored[i]),
                local_mass_frame: Some(mass_frame_words(masses[i].local_mass_frame)),
                body: Some([0; 44]),
                inertia: (mode != BoardMotion::Static).then(|| inertia_words(masses[i].dynamics)),
            };
            set_part_transform(&mut part, pose_words(authored[i]));
            part
        });
        let mut hook_part = PartPose {
            transform: pose_words(RetailAffineTransform::IDENTITY),
            local_mass_frame: None,
            body: Some([0; 44]),
            inertia: None,
        };
        set_board_transform(&mut parts, &mut hook_part, pose_words(spawn));
        let bodies = core::array::from_fn(|i| {
            initialized_body(&parts[i], masses[i].dynamics, simulation, mode)
        });
        // CreateHookDrive 82C0D330 creates its own STATIC_BODY target. It does
        // not alias the deck or a global world body. Its mass calculation is
        // irrelevant to physical response because this body has no inertia.
        let hook = BoardHook {
            body: initialized_body(&hook_part, ZERO_INERTIA, simulation, BoardMotion::Static),
            drive: HookDriveState::initial(),
        };
        Self {
            bodies,
            mass_frames: masses.map(|mass| mass.local_mass_frame),
            hook,
            step: BoardStep::default(),
            forces: BoardForceQueue::default(),
            collision_group: 4,
        }
    }

    pub fn collision_group(&self) -> u32 {
        self.collision_group
    }
    ///All seven native board parts share the current assembly group.
    pub fn set_collision_group(&mut self, group: u32) {
        self.collision_group = group
    }

    pub fn bodies(&self) -> &[BodySnapshot; BODY_COUNT] {
        &self.bodies
    }

    /// Physical portion of82C05F50/82C0D680. Restore the authored part poses,
    /// apply the requested stance and board transform, then clear rates. State
    /// flags, cooldown, inertias and target-body rates retain their own owners.
    pub fn reset_physical(
        &mut self,
        authored: [RetailAffineTransform; BODY_COUNT],
        mut target: RetailAffineTransform,
        processed_flags: u32,
        gravity: Vector3,
    ) {
        let mut parts = core::array::from_fn(|i| PartPose {
            transform: pose_words(authored[i]),
            local_mass_frame: Some(mass_frame_words(self.mass_frames[i])),
            body: Some(body_pose_words(self.bodies[i].rates)),
            inertia: Some(inertia_words(self.bodies[i].inertia)),
        });
        for i in [6, 0, 1, 2, 3, 4, 5] {
            set_part_transform(&mut parts[i], pose_words(authored[i]));
        }
        if processed_flags & 0x0010_0000 != 0 {
            for i in [0, 2] {
                target.basis.columns[i] = target.basis.columns[i].map(|v| -v);
            }
        }
        let mut hook = PartPose {
            transform: pose_words(self.hook_transform()),
            local_mass_frame: None,
            body: Some(body_pose_words(self.hook.body.rates)),
            inertia: None,
        };
        set_board_transform(&mut parts, &mut hook, pose_words(target));
        for (body, part) in self.bodies.iter_mut().zip(&parts) {
            copy_pose(part, &mut body.rates);
            body.rates.linear_velocity = Vector3::ZERO;
            body.rates.angular_velocity = Vector3::ZERO;
            body.rates.force_acceleration = gravity;
            body.rates.torque_acceleration = Vector3::ZERO;
            body.rates.world_inverse_inertia =
                world_inverse_inertia(body.rates.basis, body.inertia.inverse_tensor);
        }
        copy_pose(&hook, &mut self.hook.body.rates);
        self.forces.clear();
        // Completed contacts belong to the old pose; host storage can be
        // discarded without rebuilding physical bodies or their constraints.
        self.step = BoardStep::default();
    }

    /// Move the complete board assembly while preserving live rates, forces,
    /// contacts, and solver ownership. This is the C055F0/C0B2C8 possession
    /// position operation, distinct from the full physical reset above.
    pub fn set_transform(&mut self, target: RetailAffineTransform) {
        let authored = self.part_transforms();
        let mut parts = core::array::from_fn(|i| PartPose {
            transform: pose_words(authored[i]),
            local_mass_frame: Some(mass_frame_words(self.mass_frames[i])),
            body: Some(body_pose_words(self.bodies[i].rates)),
            inertia: Some(inertia_words(self.bodies[i].inertia)),
        });
        let mut hook = PartPose {
            transform: pose_words(self.hook_transform()),
            local_mass_frame: None,
            body: Some(body_pose_words(self.hook.body.rates)),
            inertia: None,
        };
        set_board_transform(&mut parts, &mut hook, pose_words(target));
        for (body, part) in self.bodies.iter_mut().zip(&parts) {
            copy_pose(part, &mut body.rates);
            body.rates.world_inverse_inertia =
                world_inverse_inertia(body.rates.basis, body.inertia.inverse_tensor);
        }
        copy_pose(&hook, &mut self.hook.body.rates);
    }

    /// Gameplay may publish actual velocities/accumulators or physical mode
    /// changes here. Rendering should use the immutable view or part poses.
    pub fn bodies_mut(&mut self) -> &mut [BodySnapshot; BODY_COUNT] {
        &mut self.bodies
    }

    pub fn hook(&self) -> &BoardHook {
        &self.hook
    }

    /// Animation/state services own the hook target and live drive parameters.
    pub fn hook_mut(&mut self) -> &mut BoardHook {
        &mut self.hook
    }

    pub fn forces(&self) -> &BoardForceQueue {
        &self.forces
    }

    pub fn contact_reports(&self) -> &[super::contact_feedback::BoardContactReport] {
        self.step.contact_reports()
    }

    /// Original post-solver observers consume these completed shared rows.
    pub fn solved_reactions(&self) -> &[super::rigid_body::RetailReactionCorrections] {
        self.step.solved_reactions()
    }
    pub fn solved_contacts(&self) -> &[super::contact_solver::RetailContactJacobian] {
        self.step.solved_contacts()
    }

    pub fn forces_mut(&mut self) -> &mut BoardForceQueue {
        &mut self.forces
    }

    /// Queue clearing is a separate gameplay phase, as in GeneralUpdate
    /// 82C02360. Advancing physics does not silently consume its ownership.
    pub fn clear_forces(&mut self) {
        self.forces.clear();
    }

    pub fn advance(
        &mut self,
        collisions: &[BoardCollision],
        truck_targets: [f32; 2],
        settings: BoardStepSettings,
    ) {
        self.step.advance(
            &mut self.bodies,
            &mut self.hook,
            &self.forces,
            collisions,
            truck_targets,
            settings,
        );
    }

    ///The skater, board and hook constraints exchange reactions in every
    ///iteration; this never advances the skeleton in a separate solver.
    pub fn advance_attached(
        &mut self,
        collisions: &[BoardCollision],
        truck_targets: [f32; 2],
        settings: BoardStepSettings,
        attached: AttachedStep<'_>,
    ) {
        self.step.advance_attached(
            &mut self.bodies,
            &mut self.hook,
            &self.forces,
            collisions,
            truck_targets,
            settings,
            attached,
        );
    }

    /// The leading attached bodies of the next shared solve that belong to this board's skater.
    pub fn set_owned_attached(&mut self, count: usize) {
        self.step.owned_attached = count;
    }

    /// Live center-of-mass pose for collision and other physical consumers.
    pub fn body_transform(&self, id: BodyId) -> RetailAffineTransform {
        rates_transform(self.bodies[id.index()].rates)
    }

    /// Live visual/collision part poses, including each mass-frame offset.
    /// This derives poses from the integrated bodies rather than stale spawn
    /// matrices, and never writes physics from presentation.
    pub fn part_transforms(&self) -> [RetailAffineTransform; BODY_COUNT] {
        core::array::from_fn(|i| {
            let part = PartPose {
                transform: pose_words(rates_transform(self.bodies[i].rates)),
                local_mass_frame: Some(mass_frame_words(self.mass_frames[i])),
                body: Some(body_pose_words(self.bodies[i].rates)),
                inertia: None,
            };
            transform_from_words(part_transform(&part))
        })
    }

    pub fn hook_transform(&self) -> RetailAffineTransform {
        rates_transform(self.hook.body.rates)
    }

    /// Publish an animation target while retaining the hook's zero physical
    /// response and its separate drive state.
    pub fn set_hook_transform(&mut self, requested: RetailAffineTransform) {
        let mut part = PartPose {
            transform: pose_words(self.hook_transform()),
            local_mass_frame: None,
            body: Some(body_pose_words(self.hook.body.rates)),
            inertia: None,
        };
        set_part_transform(&mut part, pose_words(requested));
        copy_pose(&part, &mut self.hook.body.rates);
    }
}

const ZERO_INERTIA: RetailInertiaDynamics = RetailInertiaDynamics {
    inverse_tensor: Vector3::ZERO,
    inverse_mass: 0.0,
    spherical: 0.0,
    maximum_linear_velocity: 0.0,
    maximum_angular_velocity: 0.0,
    linear_drag: 0.0,
    angular_drag: 0.0,
};

pub(crate) fn initialized_body(
    part: &PartPose,
    inertia: RetailInertiaDynamics,
    simulation: RetailSimulationStep,
    mode: BoardMotion,
) -> BodySnapshot {
    let is_static = mode == BoardMotion::Static;
    let inertia = if is_static { ZERO_INERTIA } else { inertia };
    let mut rates = RetailBodyRates {
        orientation: RetailQuaternion::IDENTITY,
        basis: RetailAffineTransform::IDENTITY.basis,
        world_inverse_inertia: Basis3 {
            columns: [[0.0; 3]; 3],
        },
        position: Vector3::ZERO,
        linear_velocity: Vector3::ZERO,
        angular_velocity: Vector3::ZERO,
        force_acceleration: if is_static {
            Vector3::ZERO
        } else {
            simulation.gravity_acceleration
        },
        torque_acceleration: Vector3::ZERO,
        kinetic_energy: if is_static { 0.0 } else { f32::MAX },
        cool_down: if mode == BoardMotion::Active {
            0
        } else {
            simulation.cool_down
        },
    };
    copy_pose(part, &mut rates);
    rates.world_inverse_inertia = world_inverse_inertia(rates.basis, inertia.inverse_tensor);
    BodySnapshot {
        state_flags: mode.flags(),
        rates,
        inertia,
    }
}

fn copy_pose(part: &PartPose, rates: &mut RetailBodyRates) {
    let words = part
        .body
        .as_ref()
        .expect("transient pose adapter has a body");
    let f = |i| f32::from_bits(words[i]);
    rates.orientation = RetailQuaternion {
        x: f(0),
        y: f(1),
        z: f(2),
        w: f(3),
    };
    rates.position = Vector3::new(f(4), f(5), f(6));
    rates.basis = Basis3 {
        columns: [
            [f(16), f(17), f(18)],
            [f(20), f(21), f(22)],
            [f(24), f(25), f(26)],
        ],
    };
}

fn rates_transform(rates: RetailBodyRates) -> RetailAffineTransform {
    RetailAffineTransform {
        basis: rates.basis,
        translation: rates.position,
    }
}

fn body_pose_words(rates: RetailBodyRates) -> [u32; 44] {
    let mut words = [0; 44];
    let q = rates.orientation;
    words[..4].copy_from_slice(&[q.x, q.y, q.z, q.w].map(f32::to_bits));
    words[4..7]
        .copy_from_slice(&[rates.position.x, rates.position.y, rates.position.z].map(f32::to_bits));
    for (axis, column) in rates.basis.columns.into_iter().enumerate() {
        words[16 + axis * 4..19 + axis * 4].copy_from_slice(&column.map(f32::to_bits));
    }
    words
}

fn inertia_words(inertia: RetailInertiaDynamics) -> [u32; 10] {
    [
        inertia.inverse_tensor.x,
        inertia.inverse_tensor.y,
        inertia.inverse_tensor.z,
        0.0,
        inertia.inverse_mass,
        inertia.spherical,
        inertia.maximum_linear_velocity,
        inertia.maximum_angular_velocity,
        inertia.linear_drag,
        inertia.angular_drag,
    ]
    .map(f32::to_bits)
}

fn mass_frame_words(frame: RetailLocalMassFrame) -> PoseMatrix {
    pose_words(RetailAffineTransform {
        basis: frame.basis,
        translation: frame.translation,
    })
}

fn pose_words(transform: RetailAffineTransform) -> PoseMatrix {
    let mut words = [0; 16];
    for (axis, column) in transform.basis.columns.into_iter().enumerate() {
        words[axis * 4..axis * 4 + 3].copy_from_slice(&column.map(f32::to_bits));
    }
    let t = transform.translation;
    words[12..15].copy_from_slice(&[t.x, t.y, t.z].map(f32::to_bits));
    words
}

fn transform_from_words(words: PoseMatrix) -> RetailAffineTransform {
    let f = |i| f32::from_bits(words[i]);
    RetailAffineTransform {
        basis: Basis3 {
            columns: [[f(0), f(1), f(2)], [f(4), f(5), f(6)], [f(8), f(9), f(10)]],
        },
        translation: Vector3::new(f(12), f(13), f(14)),
    }
}

#[cfg(test)]
#[path = "tests/board_runtime.rs"]
mod tests;
