//! Ground Enter `82D37538` and Exit `82D37B20` lifecycle order.

use crate::{
    physics::{
        force_queue::{BoardForceQueue, QueuedPointForce},
        manual::state::ManualState,
    },
    riding::{
        grounded::{
            drag::{BodyInertias, DragBindingError, DragSelection},
            manual_entry::{
                ManualGroundBodies, ManualGroundInput, ManualGroundProjection, enter_ground,
            },
        },
        pumping::state::PumpingState,
        speed_wobble::SpeedWobbleState,
    },
};

use super::data::PhysicsGroundState;

#[derive(Clone, Copy, Debug)]
pub struct GroundEnterInput {
    pub previous_category_2516: u32,
    pub previous_state_2504: u32,
    pub powerslide_exit_scale: f32,
    pub manual_ground: ManualGroundInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundEnterStage {
    SetBoardStandard,
    InitializeSkeleton,
    SetCollisionState,
    ResetExternalObjects,
    InitializeBoardSpeed,
    EnterManual,
    EnterFromAir,
    BuildLandingOnDeckForce,
    SetControllerMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GroundEnterError<S, P> {
    Service { stage: GroundEnterStage, source: S },
    ManualProjection { stage: GroundEnterStage, source: P },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundEnterResult {
    pub landing_force_attempted: bool,
    pub landing_force_queued: bool,
}

/// Required object operations surrounding the state-owned resets. None have a
/// default because their complete side effects are part of Ground entry.
pub trait GroundEnterServices {
    type Error;

    fn set_board_standard_82c03af8(&mut self) -> Result<(), Self::Error>;
    fn initialize_ground_skeleton(&mut self) -> Result<(), Self::Error>;
    fn set_collision_state_82d911a8(&mut self, state: u32) -> Result<(), Self::Error>;
    fn reset_ground_external_objects(&mut self) -> Result<(), Self::Error>;
    /// Full `82D375D0..82D376CC` board transform/velocity projection and target
    /// speed initialization, preserving native VMX arithmetic.
    fn initialize_board_speed_82d375d0(&mut self) -> Result<(), Self::Error>;
    fn enter_from_air_82bf2f18(&mut self) -> Result<(), Self::Error>;
    /// Full tag-19 producer `82D377AC..82D378A8`.
    fn landing_on_deck_force(&mut self) -> Result<QueuedPointForce, Self::Error>;
    fn set_controller_mode_one(&mut self) -> Result<(), Self::Error>;
}

pub struct GroundEntryComponents<'a> {
    pub pumping: &'a mut PumpingState,
    pub speed_wobble: &'a mut SpeedWobbleState,
    pub manual: &'a mut ManualState,
    pub force_queue: &'a mut BoardForceQueue,
}

/// Runs every verified Ground entry operation in native order. The service
/// boundaries are unresolved engine/numerical functions, not optional hooks.
pub fn enter<S, B, P>(
    state: &mut PhysicsGroundState,
    components: GroundEntryComponents<'_>,
    input: GroundEnterInput,
    bodies: &mut B,
    projection: &mut P,
    services: &mut S,
) -> Result<GroundEnterResult, GroundEnterError<S::Error, P::Error>>
where
    S: GroundEnterServices,
    B: ManualGroundBodies,
    P: ManualGroundProjection,
{
    enter_call(
        GroundEnterStage::SetBoardStandard,
        services.set_board_standard_82c03af8(),
    )?;
    enter_call(
        GroundEnterStage::InitializeSkeleton,
        services.initialize_ground_skeleton(),
    )?;
    enter_call(
        GroundEnterStage::SetCollisionState,
        services.set_collision_state_82d911a8(6),
    )?;

    state.word_2560 = 0;
    state.word_2564 = 0;
    state.flag_2708 = false;
    state.controls_latched_2725 = false;
    state.captured_position_valid_2726 = false;
    state.was_pinning_2728 = false;
    state.flag_2731 = false;
    state.manual_correction_2732 = false;
    state.manual_opposition_2733 = false;

    enter_call(
        GroundEnterStage::ResetExternalObjects,
        services.reset_ground_external_objects(),
    )?;
    enter_call(
        GroundEnterStage::InitializeBoardSpeed,
        services.initialize_board_speed_82d375d0(),
    )?;
    components.speed_wobble.reset();
    enter_ground(
        components.manual,
        input.previous_category_2516,
        input.powerslide_exit_scale,
        input.manual_ground,
        bodies,
        projection,
    )
    .map_err(|source| GroundEnterError::ManualProjection {
        stage: GroundEnterStage::EnterManual,
        source,
    })?;

    if input.previous_category_2516 == 200 {
        enter_call(
            GroundEnterStage::EnterFromAir,
            services.enter_from_air_82bf2f18(),
        )?;
    }
    let mut result = GroundEnterResult {
        landing_force_attempted: false,
        landing_force_queued: false,
    };
    if input.previous_state_2504 == 503 {
        let mut force = enter_call(
            GroundEnterStage::BuildLandingOnDeckForce,
            services.landing_on_deck_force(),
        )?;
        force.tag = 19;
        result.landing_force_attempted = true;
        result.landing_force_queued = components.force_queue.append(force);
    }

    state.straighten_scale_2672 = if input.previous_state_2504 == 101 {
        f32::from_bits(0x3e23_d70a)
    } else {
        1.0
    };
    components.pumping.reset();
    enter_call(
        GroundEnterStage::SetControllerMode,
        services.set_controller_mode_one(),
    )?;

    state.steering_push_scalar_2640 = 1.0;
    state.steering_damped_turn_2644 = 0.0;
    state.elapsed_2648 = 0.0;
    state.collision_countdown_2652 = 0.0;
    state.vector_2592 = [0.0; 4];
    state.vector_2608 = [0.0; 4];
    state.anti_flip_torque_2624 = [0.0; 4];
    state.hang_detection_frames_2740 = 0;
    state.hang_force_frames_2744 = 0;
    state.hung_wipeout_frames_2748 = 0;
    state.anti_flip_nudge_frames_2752 = 0;
    state.flag_2721 = false;
    state.anti_flip_nudge_applied_2723 = false;
    state.push_suppressed_2730 = false;
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundExitStage {
    SetWheelMaterials,
    RestoreAnimatedBoardVelocity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GroundExitError<E> {
    Drag(DragBindingError),
    Service { stage: GroundExitStage, source: E },
}

pub trait GroundExitServices {
    type Error;

    fn set_default_wheel_materials(&mut self) -> Result<(), Self::Error>;
    /// Full `82D37BAC..82D37C30` jump-velocity clamp and board velocity write.
    fn restore_animated_board_velocity(&mut self, vector_2688: [f32; 4])
    -> Result<(), Self::Error>;
}

pub fn exit<S: GroundExitServices>(
    state: &mut PhysicsGroundState,
    pumping: &mut PumpingState,
    inertias: &mut BodyInertias<'_>,
    services: &mut S,
) -> Result<(), GroundExitError<S::Error>> {
    state.steering_damped_turn_2644 = 0.0;
    state.steering_push_scalar_2640 = 1.0;
    pumping.reset();
    inertias
        .set_linear_drag(0.0, DragSelection::AllParts)
        .map_err(GroundExitError::Drag)?;
    services
        .set_default_wheel_materials()
        .map_err(|source| GroundExitError::Service {
            stage: GroundExitStage::SetWheelMaterials,
            source,
        })?;
    if state.flag_2708 {
        services
            .restore_animated_board_velocity(state.vector_2688)
            .map_err(|source| GroundExitError::Service {
                stage: GroundExitStage::RestoreAnimatedBoardVelocity,
                source,
            })?;
    }
    Ok(())
}

fn enter_call<T, S, P>(
    stage: GroundEnterStage,
    result: Result<T, S>,
) -> Result<T, GroundEnterError<S, P>> {
    result.map_err(|source| GroundEnterError::Service { stage, source })
}
