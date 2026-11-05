//! Brake/push production82D38EB4..8F08 and submission82D39194..91B0.
//! These are two distinct slices of Ground UpdateSkateboard, not an entire
//! ground update: intervening native force/manual producers must execute.
use crate::{
    math::Vector3,
    physics::{
        force_queue::{BoardForceQueue, QueuedPointForce},
        manual::controller::ManualEffect,
    },
    riding::{
        braking::{BrakeInput, BrakeSettings, calculate_braking},
        push::{PushAcceleration, PushInput, PushLimits, calculate_acceleration},
    },
};

/// Actual ProcessedPhysIn fields consumed by these two native calculations.
/// No raw button mapping, recomputed mass, normalized axis or host timestep.
#[derive(Clone, Copy, Debug)]
pub struct GroundPropulsionInput {
    pub flags_2468: u32,
    pub flags_2472: u32,
    ///+2608, produced by Skeleton's push attribute path82BDAC24/2C.
    pub target_speed: f32,
    ///+2612, copied by ProcessInput82DB4878 from player1800->4 field168.
    pub signed_speed: f32,
    ///+2616, separate from the signed projection above.
    pub absolute_body_speed: f32,
    ///+2660, consumed directly. Existing mass-producer attribution82C0140C
    ///still needs a fresh containing-body audit; do not recompute here.
    pub scalar_2660: f32,
    ///+2604, unlike Ground's fixed pumping timestep.
    pub timestep: f32,
    ///+2728, Skeleton attribute scalar consumer82BDB0E8.
    pub brake_input: f32,
    ///+352 and+368 are distinct effective axes, used verbatim.
    pub push_direction: Vector3,
    pub brake_direction: Vector3,
    /// Processed+2544 -> collection+4 -> layout+264.
    pub surface_braking_factor: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GroundPropulsionSettings {
    /// Ground+32 toolkit offsets20/24/28, cached by82D94D08/1C/34.
    pub braking: BrakeSettings,
    /// Toolkit+16 from physics_push layout+8 at82D94CF4.
    pub maximum_pushable_speed: f32,
    /// Processed+2548 -> selected collection+4 -> layout+48/+52.
    /// These are separate from the pumping fields at that mode binding.
    pub mode_speed_changes: [f32; 2],
}

/// Native stack results retained until the later force-submission phase.
#[derive(Clone, Copy, Debug)]
pub struct GroundPropulsion {
    pub braking: QueuedPointForce,
    pub push: PushAcceleration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropulsionSubmission {
    BrakingAndPush([bool; 2]),
    ManualCorrection(bool),
}

/// Run only after the earlier velocity/collision branches select82D38EB4.
/// The force helpers run even if the later manual branch suppresses submission.
/// Publishes Ground+2730 after push calculation, including clearing old state.
pub fn calculate(
    input: GroundPropulsionInput,
    settings: GroundPropulsionSettings,
    push_suppressed_2730: &mut u8,
) -> GroundPropulsion {
    let braking = calculate_braking(
        BrakeInput {
            flags_2468: input.flags_2468,
            input_2728: input.brake_input,
            signed_speed: input.signed_speed,
            absolute_body_speed: input.absolute_body_speed,
            surface_factor: input.surface_braking_factor,
            direction: input.brake_direction,
        },
        settings.braking,
    );
    let push = calculate_acceleration(
        PushInput {
            flags_2468: input.flags_2468,
            flags_2472: input.flags_2472,
            target_speed: input.target_speed,
            current_speed: input.signed_speed,
            absolute_body_speed: input.absolute_body_speed,
            scale: input.scalar_2660,
            delta_seconds: input.timestep,
            direction: input.push_direction,
        },
        PushLimits {
            maximum_pushable_speed: settings.maximum_pushable_speed,
            low_speed_change: settings.mode_speed_changes[0],
            high_speed_change: settings.mode_speed_changes[1],
        },
    );
    *push_suppressed_2730 = u8::from(push.suppressed);
    GroundPropulsion { braking, push }
}

impl GroundPropulsion {
    /// Call at82D39194 after native correction82D39338 and any tag4/5 writes,
    /// before angular changes/tag6/1/8/16. Queues both records, even zero forces.
    /// A true actual CalcManualEffect output80 selects the alternate tag7 path
    /// and queues its corrective force+48/point+64 instead. Output81 is not
    /// this selector.82D94A20 writes
    /// no r9, preserving the output80 byte through the intervening drag call.
    /// Returns each append result; the native21-entry limit may drop a record.
    pub fn submit(
        self,
        manual: &ManualEffect,
        queue: &mut BoardForceQueue,
    ) -> PropulsionSubmission {
        if manual.correction_active {
            let force = manual.corrective_force_world;
            let point = manual.corrective_point_body;
            return PropulsionSubmission::ManualCorrection(queue.append(QueuedPointForce {
                tag: 7,
                force_world: Vector3::new(force[0], force[1], force[2]),
                point_body: Vector3::new(point[0], point[1], point[2]),
            }));
        }
        let brake = queue.append(self.braking);
        let push = queue.append(QueuedPointForce {
            tag: 3,
            force_world: self.push.vector,
            point_body: self.push.local_point,
        });
        PropulsionSubmission::BrakingAndPush([brake, push])
    }
}
