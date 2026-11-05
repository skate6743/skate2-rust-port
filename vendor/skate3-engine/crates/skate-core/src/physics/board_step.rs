//! The connected-board path through TU3's simulation frame.
//!
//! Sources: simulation_job_setup_resolution.json (82DC35E0),
//! drive_solver_handoff_resolution.json (82DC3098 / 82AE27D0 / 82AE6590).
//! This owns compilation, shared reactions and integration. Collision queries
//! and ground/action-state force production have separate owners.
use crate::math::Vector3;

use super::{
    assembly::{BoardConstraints, BoardHook, BodySnapshot, prepare_drive_frames},
    board::{BODY_COUNT, BodyId},
    contact::{RetailContactBodyState, RetailContactInput, generate_contact},
    contact_feedback::{BoardContactReport, board_reports},
    contact_solver::{RetailContactJacobian, build_contact_jacobian},
    drive_frames::RetailAffineTransform,
    drive_parameters::RetailDriveDynamics,
    drive_solver::RetailDriveRows,
    force_queue::BoardForceQueue,
    point_force::RetailForceAccumulator,
    rigid_body::{
        RetailReactionCorrections, RetailSimulationStep, integrate_body_rates,
        pack_world_inverse_inertia,
    },
    solver::{JointConstraint, solve_constraints},
};

#[cfg(test)]
const WORLD_REACTION: usize = BODY_COUNT + 1;
#[cfg(test)]
const REACTION_COUNT: usize = BODY_COUNT + 2;
///Board parts0..6 and their separate hook7 precede the physical skater.
pub const ATTACHED_REACTION_BASE: usize = BODY_COUNT + 1;

///Other assemblies participating in this SAME solve. References point to the
///actual bodies owned by the physical skeleton and its hooks. The caller builds
///their rows after publishing this tick's physical drive targets and forces.
pub struct AttachedStep<'a> {
    pub bodies: Vec<&'a mut BodySnapshot>,
    pub contacts: &'a mut [RetailContactJacobian],
    pub joints: &'a mut [JointConstraint],
    pub drives: &'a mut [RetailDriveRows],
}
impl AttachedStep<'_> {
    pub fn world_reaction(&self) -> usize {
        ATTACHED_REACTION_BASE + self.bodies.len()
    }
}

/// Narrow-phase output, in the collision provider's original A/B order.
/// Materials must already be combined by `combine_contact_materials`.
#[derive(Clone, Copy, Debug)]
pub struct BoardCollision {
    pub body_a: CollisionBody,
    pub body_b: CollisionBody,
    pub contact: RetailContactInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionBody {
    Board(BodyId),
    /// Index in AttachedStep::bodies, referencing the actual physical body.
    Attached(usize),
    StaticWorld,
}

impl CollisionBody {
    pub fn contact_id(self) -> u32 {
        match self {
            Self::Board(id) => id.index() as u32,
            Self::Attached(index) => u32::try_from(ATTACHED_REACTION_BASE + index)
                .expect("attached contact body index exceeds host storage"),
            Self::StaticWorld => u32::MAX,
        }
    }

    pub fn from_contact_id(id: u32) -> Self {
        if id == u32::MAX {
            Self::StaticWorld
        } else if id < BODY_COUNT as u32 {
            Self::Board(BodyId::ORDER[id as usize])
        } else {
            assert!(
                id >= ATTACHED_REACTION_BASE as u32,
                "board target has no collision volume"
            );
            Self::Attached(id as usize - ATTACHED_REACTION_BASE)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoardStepSettings {
    pub simulation: RetailSimulationStep,
    /// Loaded from Simulation+176 by 82DC3098. Not a fixed solver constant.
    pub iterations: u32,
    pub base_truck_transforms: [RetailAffineTransform; 2],
    pub truck_dynamics: RetailDriveDynamics,
    pub force_point_y_offset: f32,
}

/// Reuses the contact workspace between ticks. Seven board bodies and the
/// separate hook share solver reactions. This does not assign islands or implement the
/// native activation partition for multiple disconnected assemblies.
#[derive(Default)]
pub struct BoardStep {
    contacts: Vec<RetailContactJacobian>,
    reports: Vec<BoardContactReport>,
    /// The leading attached bodies owned by this board's skater (its skeleton and drive
    /// targets). CreateContactSpyList 0x82768418 reports no contact between two bodies of one
    /// owner (Body+32), so the board's reports leave these out.
    pub owned_attached: usize,
    reactions: Vec<RetailReactionCorrections>,
    solved_reactions: Vec<RetailReactionCorrections>,
    pub diagnostic_capture: bool,
    diagnostic_tick: u64,
    pub diagnostic_snapshot: Option<String>,
}

impl BoardStep {
    /// Read-only solver output, before DynamicUpdate consumes/clears it.
    /// Host-owned bodies use this to transfer reactions across solver worlds.
    pub fn solved_reactions(&self) -> &[RetailReactionCorrections] {
        &self.solved_reactions
    }
    pub fn contact_reports(&self) -> &[BoardContactReport] {
        &self.reports
    }
    pub fn solved_contacts(&self) -> &[RetailContactJacobian] {
        &self.contacts
    }
    /// The caller supplies current poses, processed steering targets, queued
    /// forces and narrow-phase contacts. It owns the separate force-queue reset
    /// in GeneralUpdate (82C02360); consuming forces does not clear the queue.
    pub fn advance(
        &mut self,
        bodies: &mut [BodySnapshot; BODY_COUNT],
        hook: &mut BoardHook,
        forces: &BoardForceQueue,
        collisions: &[BoardCollision],
        truck_targets: [f32; 2],
        settings: BoardStepSettings,
    ) {
        self.advance_attached(
            bodies,
            hook,
            forces,
            collisions,
            truck_targets,
            settings,
            AttachedStep {
                bodies: Vec::new(),
                contacts: &mut [],
                joints: &mut [],
                drives: &mut [],
            },
        );
    }

    pub fn advance_attached(
        &mut self,
        bodies: &mut [BodySnapshot; BODY_COUNT],
        hook: &mut BoardHook,
        forces: &BoardForceQueue,
        collisions: &[BoardCollision],
        truck_targets: [f32; 2],
        settings: BoardStepSettings,
        mut attached: AttachedStep<'_>,
    ) {
        // StartFrame writes both timing fields (82DC2FB8). Use one dt for row
        // construction and integration, and derive the reciprocal from it.
        let mut simulation = settings.simulation;
        assert!(simulation.time_step.is_finite() && simulation.time_step > 0.0);
        simulation.frequency = 1.0 / simulation.time_step;
        for collision in collisions {
            assert_ne!(collision.body_a, collision.body_b);
        }

        apply_deck_forces(bodies, forces, settings.force_point_y_offset);
        let frames = prepare_drive_frames(settings.base_truck_transforms, truck_targets, hook);
        let world_reaction = attached.world_reaction();
        self.reactions
            .resize(world_reaction + 1, RetailReactionCorrections::default());
        self.reactions.fill(RetailReactionCorrections::default());
        self.contacts.clear();
        self.reports.clear();
        self.solved_reactions.clear();
        if bodies
            .iter()
            .chain(core::iter::once(&hook.body))
            .chain(attached.bodies.iter().map(|body| &**body))
            .all(|body| body.state_flags & 4 == 0)
        {
            return;
        }
        for collision in collisions {
            // Copy force/torque workspaces AFTER force accumulation. Otherwise
            // ContactBatchBuild cannot oppose this frame's applied forces.
            let contact = generate_contact(
                collision.contact,
                contact_body(collision.body_a, bodies, &attached.bodies, world_reaction),
                contact_body(collision.body_b, bodies, &attached.bodies, world_reaction),
            );
            self.contacts
                .push(build_contact_jacobian(contact, simulation.time_step));
        }
        let mut constraints = BoardConstraints::build(
            bodies,
            hook,
            frames,
            settings.truck_dynamics,
            simulation.time_step,
        );
        let board_contacts = self.contacts.len();
        let board_joints = constraints.joints.len();
        let board_drives = constraints.drives.len();
        self.contacts.extend_from_slice(attached.contacts);
        constraints.joints.extend_from_slice(attached.joints);
        constraints.drives.extend_from_slice(attached.drives);
        solve_constraints(
            &mut self.contacts,
            &mut constraints.joints,
            &mut constraints.drives,
            &mut self.reactions,
            settings.iterations,
        );
        self.diagnostic_tick = self.diagnostic_tick.wrapping_add(1);
        if self.diagnostic_capture && self.diagnostic_tick % 6 == 0 {
            let deck = BodyId::Deck.index();
            self.diagnostic_snapshot = Some(format!(
                "tick={} body={:?} hook={:?} reaction={:?} joints={:?} drives={:?}",
                self.diagnostic_tick,
                bodies[deck],
                hook,
                self.reactions[deck],
                constraints
                    .joints
                    .iter()
                    .enumerate()
                    .filter(|(_, j)| j.reaction_a == deck || j.reaction_b == deck)
                    .map(|(i, j)| (
                        i,
                        j.reaction_a,
                        j.reaction_b,
                        j.jacobian.words.map(f32::from_bits)
                    ))
                    .collect::<Vec<_>>(),
                constraints
                    .drives
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.frame_a_body.reaction_index == deck
                        || d.frame_b_body.reaction_index == deck)
                    .collect::<Vec<_>>(),
            ));
        }
        self.solved_reactions.clone_from(&self.reactions);
        // Every solver family finishes before ANY body integrates. The native
        // job tree places BatchIntegrator after all island solver jobs.
        for (body, reaction) in bodies
            .iter_mut()
            .chain(core::iter::once(&mut hook.body))
            .chain(attached.bodies.iter_mut().map(|body| &mut **body))
            .zip(&mut self.reactions)
        {
            if body.state_flags & 4 != 0 {
                body.rates =
                    integrate_body_rates(body.rates, body.inertia, simulation, *reaction).state;
            }
            // DynamicUpdate clears all four working vectors, not just the
            // velocity-producing pair. Accumulated rows are rebuilt next tick.
            *reaction = RetailReactionCorrections::default();
        }
        // ContactSpiesJob follows integration (82DC35E0). Step_Solver3 then
        // publishes reports before the physical Adjust/Output phases.
        let attached_refs: Vec<&BodySnapshot> =
            attached.bodies.iter().map(|body| &**body).collect();
        board_reports::collect(
            &mut self.reports,
            &self.contacts,
            bodies,
            &attached_refs,
            self.owned_attached,
            simulation.frequency,
        );
        //Physical output/spy consumers observe the rows from the shared solve.
        attached
            .contacts
            .clone_from_slice(&self.contacts[board_contacts..]);
        attached
            .joints
            .clone_from_slice(&constraints.joints[board_joints..]);
        attached
            .drives
            .clone_from_slice(&constraints.drives[board_drives..]);
    }
}

fn apply_deck_forces(
    bodies: &mut [BodySnapshot; BODY_COUNT],
    forces: &BoardForceQueue,
    y_offset: f32,
) {
    let deck = &mut bodies[BodyId::Deck.index()];
    let result = forces.apply_to_deck(
        RetailForceAccumulator {
            force_acceleration: deck.rates.force_acceleration,
            torque_acceleration: deck.rates.torque_acceleration,
            cool_down: deck.rates.cool_down,
        },
        deck.rates.basis,
        deck.inertia.inverse_mass,
        deck.rates.world_inverse_inertia,
        y_offset,
    );
    deck.rates.force_acceleration = result.force_acceleration;
    deck.rates.torque_acceleration = result.torque_acceleration;
    deck.rates.cool_down = result.cool_down;
}

fn contact_body(
    body: CollisionBody,
    bodies: &[BodySnapshot; BODY_COUNT],
    attached: &[&mut BodySnapshot],
    world_reaction: usize,
) -> RetailContactBodyState {
    if body == CollisionBody::StaticWorld {
        return RetailContactBodyState {
            contact_body_id: u32::MAX,
            reaction_id: world_reaction as u32,
            center_of_mass: Vector3::ZERO,
            inverse_inertia_full: Vector3::ZERO,
            inverse_inertia_split: Vector3::ZERO,
            inverse_mass: 0.0,
            state: 0,
            force_acceleration: Vector3::ZERO,
            torque_acceleration: Vector3::ZERO,
            linear_velocity: Vector3::ZERO,
            angular_velocity: Vector3::ZERO,
            kinetic_energy: 0.0,
            cool_down: 0,
        };
    };
    let contact_id = body.contact_id();
    let body = match body {
        CollisionBody::Board(id) => &bodies[id.index()],
        CollisionBody::Attached(index) => &*attached[index],
        CollisionBody::StaticWorld => unreachable!(),
    };
    let rates = body.rates;
    let inertia = pack_world_inverse_inertia(rates.world_inverse_inertia);
    RetailContactBodyState {
        contact_body_id: contact_id,
        reaction_id: contact_id,
        center_of_mass: rates.position,
        inverse_inertia_full: inertia.full,
        inverse_inertia_split: inertia.split,
        inverse_mass: body.inertia.inverse_mass,
        // The physical assemblies subscribe to post-solver feedback. Bit 8
        // requests contact observations; it does not alter dynamic eligibility.
        state: body.state_flags | 8,
        force_acceleration: rates.force_acceleration,
        torque_acceleration: rates.torque_acceleration,
        linear_velocity: rates.linear_velocity,
        angular_velocity: rates.angular_velocity,
        kinetic_energy: rates.kinetic_energy,
        cool_down: rates.cool_down,
    }
}

#[cfg(test)]
#[path = "tests/board_step.rs"]
mod tests;
