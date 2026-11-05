//! Ordinary (non-animated, non-collision-response) `82D38800` path.

use crate::{
    math::Vector3,
    physics::{force_queue::QueuedPointForce, manual::controller},
    riding::{
        anti_flip, ground_force,
        grounded::propulsion::{self, PropulsionSubmission},
        heading, pumping, slide_friction, speed_model, straighten,
    },
};

use super::{
    board::service,
    board_types::{
        GroundBoardComponents, GroundBoardError, GroundBoardInput, GroundBoardServices,
        GroundBoardSettings, GroundBoardStage, GroundForceFrame,
    },
    corrections,
    data::PhysicsGroundState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryGroundResult {
    pub manual_correction: bool,
    pub terminal_force_tag: u32,
    pub terminal_force_queued: bool,
    pub speed_model_reset: bool,
}

pub(super) fn update<S: GroundBoardServices>(
    state: &mut PhysicsGroundState,
    components: GroundBoardComponents<'_, '_>,
    settings: GroundBoardSettings<'_>,
    mut input: GroundBoardInput,
    mut contact_force: QueuedPointForce,
    services: &mut S,
) -> Result<OrdinaryGroundResult, GroundBoardError<S::BoardError>> {
    let mut suppressed = 0_u8;
    let propulsion = propulsion::calculate(input.propulsion, settings.propulsion, &mut suppressed);
    state.push_suppressed_2730 = suppressed != 0;

    let (ground_a, ground_b) = ground_forces(settings.ground_force, &input.ground_force);
    input.speed_model.manual_state_276 = components.manual.angular_correction;
    let speed_force = speed_model::update(
        components.speed_model,
        settings.speed_model,
        &input.speed_model,
    );
    input.slide_friction.heading_time = state.elapsed_2648;
    // Both helpers read wheel hardness from ProcessedPhysIn+2764
    // (82D92AAC / 82D92B80), supplied by the animation profile.
    let slide_force = slide_friction::calculate(settings.slide_friction, &input.slide_friction);
    let pump_force = pumping::calculate(&input.pump_force);
    input.straighten.heading_time = state.elapsed_2648;
    let straighten = straighten::calculate(settings.straighten, &input.straighten);
    let scaled_straighten = straighten.map(|lane| lane * state.straighten_scale_2672);
    let heading = heading::calculate(
        settings.heading,
        &input.heading,
        components.heading_previous,
    );
    let anti_flip = anti_flip::calculate(settings.anti_flip, &input.anti_flip);
    state.anti_flip_torque_2624 = anti_flip;

    input.manual.powersliding = false;
    let manual = controller::calculate(
        components.manual,
        settings.manual,
        settings.manual_mode,
        &input.manual,
        services,
    )
    .map_err(GroundBoardError::Manual)?;
    state.manual_correction_2732 = manual.correction_active;
    state.manual_opposition_2733 = manual.opposing_motion_without_correction;
    let drag = input.ground_drag.calculate(settings.linear_drag);

    let terminal_queued = if manual.correction_active {
        match propulsion.submit(&manual, components.force_queue) {
            PropulsionSubmission::ManualCorrection(queued) => queued,
            PropulsionSubmission::BrakingAndPush(_) => unreachable!(),
        }
    } else {
        let push = vector4(propulsion.push.vector);
        let push_squared = service(
            GroundBoardStage::PushForceSquared,
            services.dot3(push, push),
        )?;
        if push_squared > 1.0 {
            state.flag_2721 = true;
        }
        corrections::update_anti_flip_nudge(
            state,
            input.anti_flip_nudge,
            components.force_queue,
            services,
        )
        .map_err(|source| GroundBoardError::Service {
            stage: GroundBoardStage::AntiFlipNudge,
            source,
        })?;
        if input.contact_time_2756 < settings.ground_force_contact_time_limit {
            components.force_queue.append(force_record(4, ground_a));
            components.force_queue.append(force_record(5, ground_b));
        }
        match propulsion.submit(&manual, components.force_queue) {
            PropulsionSubmission::BrakingAndPush(_) => {}
            PropulsionSubmission::ManualCorrection(_) => unreachable!(),
        }
        apply_body_vectors(
            state,
            settings.collision_response_scale,
            input.propulsion.timestep,
            scaled_straighten,
            heading,
            anti_flip,
            manual.angular_displacement,
            services,
        )?;
        if input.trajectory_state_1776_bits & 0x8000_0000 == 0 {
            components.force_queue.append(force_record(6, speed_force));
        }
        components.force_queue.append(force_record(1, slide_force));
        components.force_queue.append(force_record(8, pump_force));
        contact_force.tag = 16;
        components.force_queue.append(contact_force)
    };

    components
        .inertias
        .apply_ground_drag(drag)
        .map_err(GroundBoardError::Drag)?;
    let reset = update_speed_model_reset(
        components.speed_model,
        settings.speed_model_reset_force_squared,
        propulsion.braking.force_world,
        propulsion.push.vector,
        pump_force,
        services,
    )?;
    corrections::manage_hang_ups(state, input.hang_up, services).map_err(|source| {
        GroundBoardError::Service {
            stage: GroundBoardStage::HangUps,
            source,
        }
    })?;
    corrections::manage_halfpipe_wheel_catches(input.halfpipe_wheel_catch, services).map_err(
        |source| GroundBoardError::Service {
            stage: GroundBoardStage::HalfpipeWheelCatch,
            source,
        },
    )?;
    corrections::consider_pinning(state, input.pinning, services).map_err(|source| {
        GroundBoardError::Service {
            stage: GroundBoardStage::Pinning,
            source,
        }
    })?;
    Ok(OrdinaryGroundResult {
        manual_correction: manual.correction_active,
        terminal_force_tag: if manual.correction_active { 7 } else { 16 },
        terminal_force_queued: terminal_queued,
        speed_model_reset: reset,
    })
}

fn ground_forces(
    settings: &ground_force::GroundForceSettings,
    frame: &GroundForceFrame,
) -> ([f32; 8], [f32; 8]) {
    let mut side_scalar = 0.0;
    if frame.processed_2776 == 0.0 || frame.processed_2780 < 0.0 {
        let value = frame.ground_scalar_1232 * frame.previous_state_scalar_56;
        side_scalar = if -value >= 0.0 { 0.0 } else { value };
    }
    let common = |application_z, argument_3, argument_4| ground_force::GroundForceInput {
        argument_1: frame.argument_1_2752,
        application_z,
        argument_3,
        argument_4,
        balance: frame.balance_2720,
        surface_speed: frame.surface_speed_2656,
        axis_384: frame.axis_384,
        velocity_400: frame.velocity_400,
        axis_544: frame.axis_544,
    };
    let first = ground_force::calculate(
        settings,
        &common(
            frame.ground_scalar_1216,
            side_scalar,
            first_balance_selector(frame.balance_2720),
        ),
    );
    let second = ground_force::calculate(
        settings,
        &common(
            -frame.ground_scalar_1240,
            frame.ground_scalar_1236 * side_scalar,
            second_balance_selector(frame.balance_2720),
        ),
    );
    (first, second)
}

fn apply_body_vectors<S: GroundBoardServices>(
    state: &mut PhysicsGroundState,
    collision_scale: f32,
    timestep: f32,
    straighten: [f32; 4],
    heading: [f32; 4],
    anti_flip: [f32; 4],
    manual: [f32; 4],
    services: &mut S,
) -> Result<(), GroundBoardError<S::BoardError>> {
    if state.collision_countdown_2652 > 0.0 {
        state.collision_countdown_2652 -= timestep;
        state.vector_2592 = state.vector_2592.map(|lane| lane * collision_scale);
        service(
            GroundBoardStage::ApplyCollisionDecay,
            services.apply_vector_82c07000(state.vector_2592),
        )?;
    }
    service(
        GroundBoardStage::ApplyStraighten,
        services.apply_vector_82c07000(straighten),
    )?;
    service(
        GroundBoardStage::ApplyHeading,
        services.apply_vector_82c07000(heading),
    )?;
    service(
        GroundBoardStage::ApplyAntiFlip,
        services.apply_angular_displacement_82c075b8(anti_flip),
    )?;
    service(
        GroundBoardStage::ApplyManual,
        services.apply_angular_displacement_82c075b8(manual),
    )?;
    Ok(())
}

fn update_speed_model_reset<S: GroundBoardServices>(
    speed_model: &mut crate::riding::speed_model::SpeedModelState,
    threshold: f32,
    brake: Vector3,
    push: Vector3,
    pump: [f32; 8],
    services: &mut S,
) -> Result<bool, GroundBoardError<S::BoardError>> {
    let combined = [
        (push.x + brake.x) + pump[0],
        (push.y + brake.y) + pump[1],
        (push.z + brake.z) + pump[2],
        pump[3],
    ];
    let squared = service(
        GroundBoardStage::StrongForceSquared,
        services.dot3(combined, combined),
    )?;
    if squared > threshold {
        speed_model.flags_1360 |= 0x8000_0000;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn first_balance_selector(value: f32) -> f32 {
    if value <= 0.0 {
        if value >= -0.0 { 1.0 } else { 2.0 }
    } else {
        0.0
    }
}

fn second_balance_selector(value: f32) -> f32 {
    let nonnegative = if value <= 0.0 { 1.0 } else { 2.0 };
    if value >= -0.0 { nonnegative } else { 0.0 }
}

fn force_record(tag: u32, value: [f32; 8]) -> QueuedPointForce {
    QueuedPointForce {
        tag,
        force_world: Vector3::new(value[0], value[1], value[2]),
        point_body: Vector3::new(value[4], value[5], value[6]),
    }
}

fn vector4(value: Vector3) -> [f32; 4] {
    [value.x, value.y, value.z, 0.0]
}
