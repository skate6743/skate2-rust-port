//! Production Ground entry82D37538 and update82D37C88/82D38800.
//! State selection belongs to the player coordinator. This phase consumes its
//! current processed packet and changes the same board that the solver advances.
use super::{
    Skate3PhysicsState,
    ground_runtime::{
        GroundEntryTargets, GroundInputObservations, GroundLaunchInfo, GroundLaunchPhysical,
        GroundPhysicalFrame, GroundUpdateFrame, GroundUpdateTargets,
    },
    skater::Skate3ControllerState,
    skeleton_controller::SkeletonControllerState,
};
use skate3_core::{
    math::Vector3,
    physics::{ground_hang_geometry::HangGeometryInput, skeleton_body::SkeletonCollisionMode},
    riding::{
        collision_response::CollisionResponsePhysical, ground_contact_response::WallRidePhysical,
        grounded::state::board::GroundBoardOutcome,
    },
};

/// Edge observations are owned geometry results, not inferred from the deck.
/// The current authored world explicitly has no grind edges.
#[derive(Clone, Copy)]
pub(crate) struct GroundEdge {
    pub flags: u32,
    pub point: [f32; 4],
    pub start: Vector3,
    pub end: Vector3,
}

pub(crate) struct GroundLifecycle {
    pub skeleton_controller: SkeletonControllerState,
    /// Retained Skeleton lifecycle flag, also shared with teleport.
    /// Skeleton16388 instead belongs solely to SkeletonOutput::correction.
    pub skeleton_elapsed_16505: bool,
    pub board_animated_290: u8,
    /// Processed2724: Reset82BFA35C clears this. The original image contains
    /// no other direct scalar writer at that offset.
    pub manual_drag_2724: f32,
    pub edge: Option<GroundEdge>,
    /// A selected wall jump is retained for its actual selector continuation.
    /// It cannot be discarded as if Ground's ordinary tail had completed.
    pub pending_wall_jump: Option<GroundLaunchInfo>,
}
impl GroundLifecycle {
    pub fn new() -> Self {
        Self {
            skeleton_controller: SkeletonControllerState::new(),
            skeleton_elapsed_16505: false,
            board_animated_290: 0,
            manual_drag_2724: 0.0,
            edge: None,
            pending_wall_jump: None,
        }
    }
}

/// Typed portion of Skateboard Reset82C05F50, alongside the coordinator's
/// actual body/pose/force reset. Native timer fields8364/8368 are not written
/// by either this reset or ResetBoardBody82C0D680, so preserve activation_time.
pub(crate) fn reset_board_state(
    ground: &mut super::ground_runtime::GroundState,
    runtime: &mut super::ground_runtime::GroundRuntime,
    life: &mut GroundLifecycle,
    board_wiping_out: &mut bool,
) {
    ground.steering.deck_tilt = 0.0; //82C06048, wrapper256.
    ground.steering.targets = [0.0; 2]; //82C06060/64, body7680/7684.
    runtime.reset_board_toolkit();
    life.board_animated_290 = 0; //82C06070.
    *board_wiping_out = false; //82C0D79C..7A0 clears body8384 high2bits.
}

/// Called only on the coordinator's real transition into Ground. Entry needs
/// the toolkit already produced by this tick's PlayerInput phase.
pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Ground entry requires PlayerInput's current board toolkit")?;
    //82D37560: full SetStandard, after SetPhysicsState's possession Stop and
    //old-state Exit, before Ground disables the animation drives. Use retained
    //stock materials and publish the actual collider flags, not reset defaults.
    use skate3_core::player::offboard::board_possession::lifecycle::Effects as _;
    skater
        .board_possession_live
        .effects(
            physics,
            &mut skater.ground_lifecycle.board_animated_290,
            skater.player_input.processed.timestep_2604,
        )
        .standard_board();
    skater.board_possession_live.publish_volumes(physics);
    enter_components(
        &mut physics.board,
        &skater.player_input.processed,
        &skater.animation_input,
        toolkit,
        &mut skater.ground,
        &mut skater.ground_lifecycle,
        &mut skater.air_reckoning.state,
        &mut skater.wipeout.state,
        &mut skater.skeleton_collision,
        &mut skater.foot_ik,
        &mut skater.skeleton_output,
        &mut physics.board_wiping_out,
    )
}

/// The same entry for the reset callback while PlayerInput owns its mutable
/// processed packet. Splitting host borrows does not alter the source order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn enter_components(
    board: &mut skate3_core::physics::board_runtime::BoardRuntime,
    processed: &skate3_core::player::input_phase::ProcessedPhysicsInput,
    animation: &super::animation_input::AnimationInput,
    toolkit: &skate3_core::physics::board_toolkit::BoardToolkit,
    ground: &mut super::ground_runtime::GroundState,
    life: &mut GroundLifecycle,
    air_reckoning: &mut skate3_core::air::reckoning::AirState,
    wipeout: &mut skate3_core::player::wipeout::Requests,
    collision_mode: &mut SkeletonCollisionMode,
    ik: &mut super::foot_ik::FootIk,
    output: &mut super::skeleton_output::SkeletonOutput,
    board_wiping_out: &mut bool,
) -> Result<(), String> {
    // Skate3PhysicsState owns the meaningful high bit of Skateboard8384.
    let mut board_flags = u8::from(*board_wiping_out) << 7;
    let mut collision = |state| {
        if state != 6 {
            return Err("Ground requested a non-ground skeleton collision state".into());
        }
        life.skeleton_controller.request_ground(collision_mode)
    };
    let mut landing = |reverse| {
        output.trigger_wobble(true, reverse);
        Ok(())
    };
    let result = ground.enter(
        board,
        processed,
        animation,
        toolkit,
        GroundEntryTargets {
            foot_ik: ik,
            skeleton_elapsed_16505: &mut life.skeleton_elapsed_16505,
            reckoning_spin_angle: &mut air_reckoning.spin_angle,
            reckoning_spin_speed: &mut air_reckoning.spin_speed,
            board_flags_8384: &mut board_flags,
            board_animated_290: &mut life.board_animated_290,
            wipeout_mode: &mut wipeout.mode,
            wipeout_timer: &mut wipeout.balance,
            set_skeleton_collision_state: &mut collision,
            enter_air_landing_modifier: &mut landing,
        },
    );
    *board_wiping_out = board_flags & 0x80 != 0;
    result
}

/// Run after the Ground Reckoning preupdate and before the shared body solve.
/// The caller passes ground.steering.targets to that solve and must not clear
/// the board's force queue between this call and the solve.
pub(crate) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<GroundBoardOutcome, String> {
    super::handplant::ground_query(physics,skater);
    let selector_input = super::air_phase::selector_input(physics, skater)?;
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Ground update requires PlayerInput's current board toolkit")?;
    let life = &mut skater.ground_lifecycle;
    if life.pending_wall_jump.is_some() {
        return Err("Ground wall jump is pending its trajectory selector continuation".into());
    }
    // Native grind reset clears the complete observation block. An authored
    // world with no edges therefore has no active edge data to publish.
    let edge = life.edge.unwrap_or(GroundEdge {
        flags: 0,
        point: [0.0; 4],
        start: Vector3::ZERO,
        end: Vector3::ZERO,
    });
    let normal = raw(p.vectors_464_480_496_512_528[0]);
    let up = raw(p.vectors_544_560_592_608[0]);
    let initial_velocity = raw(p.vectors_400_416[0]);
    let mut velocity = initial_velocity;
    let mut predicted = skater.animated_skeleton.roots.predicted_board_position;
    let targets = &mut skater.skeleton_drives.targets;
    let mut move_future = |delta: Vector3| {
        targets.apply_future_deck_displacement(delta);
        predicted[0] += delta.x;
        predicted[1] += delta.y;
        predicted[2] += delta.z;
        Ok(())
    };
    let grind_context = super::air_trajectory::GrindContext::from_processed(&skater.player_input.processed, super::solve::deck_frame(&physics.board)[3]);
    let mut launch = |info: &GroundLaunchInfo| {
        let mut input = selector_input;
        input.board_vertical_velocity = info.velocity[1];
        skater.trajectory.launch(info.selector_launch(), input, &physics.world)?;
        skater.trajectory.update(input, &physics.world, grind_context)?;
        Ok(())
    };
    let physical = GroundPhysicalFrame {
        skeleton_record: &skater.animated_skeleton.record,
        wall_ride: WallRidePhysical {
            board_normal: normal,
            up,
            velocity,
            board_mass: toolkit.total_mass,
            gravity: p.gravity_2648,
            speed: p.scalar_2652,
            contact_count: p.wheel_count_2556 as i32,
        },
        collision: CollisionResponsePhysical {
            flags_2472: p.flags_2472,
            collision_displacement: raw(p.collision_pose_error_736),
            velocity: raw(p.vectors_400_416[1]),
            forward: toolkit.travel_direction,
            up,
            ground_normal: lanes(physics.riding.reckoning.ground_normal),
            time_step: p.timestep_2604,
            mass: toolkit.total_mass,
        },
        hang_geometry: HangGeometryInput {
            edge_start: edge.start,
            edge_end: edge.end,
            reference_point: xyz(edge.point),
        },
        processed_velocity: &mut velocity,
        wheel_material: &mut physics.settings.wheel_material,
        wipeout: &mut skater.wipeout.state,
        time_step: p.timestep_2604,
        launch_physical: Some(GroundLaunchPhysical {
            reckoning: &physics.riding.reckoning_frames,
            skeleton_vector_16208: skater.animated_skeleton.board_frames.local_centre_of_mass,
            skeleton_vector_16240: skater.animated_skeleton.board_frames.local_board_position,
            board_position: toolkit.deck[3],
            physical_center_of_mass: raw(p.vectors_544_560_592_608[2]),
            velocity: initial_velocity,
            angular_velocity: raw(p.prepared_jump_704),
            flags_2468: p.flags_2468,
            time_step: p.timestep_2604,
        }),
        launch_and_update: &mut launch,
    };
    let result = skater.ground.update(
        &mut skater.ground_runtime,
        &mut physics.board,
        &physics.world,
        &skater.ground_settings,
        GroundUpdateFrame {
            processed: p,
            animation: &skater.animation_input,
            riding: &physics.riding,
            skeleton: &skater.animated_skeleton,
            toolkit,
            base_trucks: physics.settings.step.base_truck_transforms,
            extra: GroundInputObservations {
                manual_drag_2724: life.manual_drag_2724,
                trajectory_state_bits: p.external_physics_1616.flags,
                edge_flags: edge.flags,
                edge_point: edge.point,
            },
        },
        physical,
        GroundUpdateTargets {
            foot_ik: &mut skater.foot_ik,
            skeleton_elapsed_16505: &mut life.skeleton_elapsed_16505,
            board_correction_pending: &mut skater.skeleton_output.correction.pending,
            // Original830BD4A0 initializer82F825F0 splats8216DEE0=-1.
            move_future_deck: &mut move_future,
        },
    );
    // These source writes occur during update. Publish even on a later branch
    // error so retrying cannot silently erase a completed physical side effect.
    skater.player_input.processed.vectors_400_416[0] = velocity.map(f32::to_bits);
    skater.animated_skeleton.roots.predicted_board_position = predicted;
    //82D37F04..F10: the Update's tail checks for a grabbable object on the same
    //PlayerGrabSpline owner as offboard, or invalidates its query.
    if result.is_ok() {
        super::ground_grab::check(skater);
    }
    result
}
fn raw(v: [u32; 4]) -> [f32; 4] {
    v.map(f32::from_bits)
}
fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}

#[cfg(any())]
#[path = "onboard_correction_tests.rs"]
mod correction_tests;
