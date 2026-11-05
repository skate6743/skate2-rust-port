//! Original Skeleton::GeneralUpdate82BDCA38 and discontinuity reset82BE2798.
//! This operates on the same physical bodies later borrowed by the solver.
use super::{
    native_arithmetic::reciprocal_estimate,
    rigid_body::RetailSimulationStep,
    skeleton_animation_record::{
        AnimationPartTransform as Transform, SkeletonAnimationRecord, compose_affine,
        transform_point,
    },
    skeleton_body::SkeletonBody,
};
use crate::math::Vector3;

///82BDCA50..CAB8: root-position derivative, two reciprocal refinements,
///then history replacement. It is distinct from animation trajectory velocity.
pub fn root_velocity(previous: &mut [f32; 4], current: [f32; 4], dt: f32) -> [f32; 4] {
    let difference: [f32; 4] = std::array::from_fn(|i| current[i] - previous[i]);
    let mut inverse = reciprocal_estimate(dt);
    for _ in 0..2 {
        inverse = inverse.mul_add((-inverse).mul_add(dt, 1.0), inverse);
    }
    let velocity = difference.map(|v| v * inverse);
    *previous = current;
    velocity
}

///82BDCB50..CC18 uses its fixed-step literal, not the caller timestep.
///The resulting displacement enters ApplyDisplacementToPart82BE7788, which
///updates force accumulators and cooldown, rather than body translations.
pub fn apply_gravity_displacement(body: &mut SkeletonBody, processed_gravity: f32) {
    let step = f32::from_bits(0x3C88_8889);
    let displacement =
        [0.0, -processed_gravity, 0.0, 0.0].map(|value| ((value * 0.5) * step) * step);
    for part in 0..24 {
        body.apply_part_displacement(part, displacement);
    }
}

pub struct ResetObservation {
    ///Skeleton16208, current adjusted animation hips translation.
    pub local_hips: [f32; 4],
    ///Skeleton16144 and16160 receive this value;16176 is zeroed.
    pub world_hips: [f32; 4],
}

///82BE2798: reset physical pose to the current mapped animation after a
///target discontinuity. Reset rates exactly here; ordinary postphysics pose
///correction must preserve rates and uses SkeletonBody::set_part_transform.
pub fn reset_to_animation(
    body: &mut SkeletonBody,
    animation: &mut SkeletonAnimationRecord,
    drive_pose: &[Transform; 24],
    animation_to_world: &Transform,
    com_frame: Transform,
    simulation: RetailSimulationStep,
) -> ResetObservation {
    for part in 0..24 {
        let world = compose_affine(animation_to_world, &drive_pose[part]);
        body.set_part_transform(part, world);
        reset_rates(body, part, simulation);
    }
    for part in [24, 25] {
        body.set_part_transform(part, com_frame);
        reset_rates(body, part, simulation);
    }
    let local_hips = drive_pose[23][3];
    let world_hips = transform_point(animation_to_world, local_hips);
    //82BEBBB0 obtains all26 current GetPartTransform results before clearing
    //their position/velocity histories and the four animation COM histories.
    let current = body.part_transforms();
    body.record.reset(&current);
    animation.reset_history();
    ResetObservation {
        local_hips,
        world_hips,
    }
}

fn reset_rates(body: &mut SkeletonBody, part: usize, simulation: RetailSimulationStep) {
    let rates = &mut body.bodies_mut()[part].rates;
    rates.linear_velocity = Vector3::ZERO;
    rates.angular_velocity = Vector3::ZERO;
    //Original Reset copies the Simulation+144 gravity vector into forceXYZ;
    //it does not clear force to zero or reset energy/cooldown.
    rates.force_acceleration = simulation.gravity_acceleration;
    rates.torque_acceleration = Vector3::ZERO;
}
