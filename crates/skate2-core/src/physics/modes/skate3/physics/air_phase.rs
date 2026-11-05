//! PhysicsAir200 production lifecycle82D34388/82D346C0/82D346A0.
//! Uses the live player, board, skeleton and selector; no alternate air solver.
mod board;
mod input;
mod skeleton;
use super::{Skate3PhysicsState, Skate3ControllerState};
pub(crate) use input::AirSettings;
pub(crate) use input::launch_info;
pub(crate) use input::selector_input;
use skate3_core::air::state::{
    AirMath, AirTrajectory, PhysicsAirMath, PhysicsAirState, angle_between_vectors,
    calculate_velocity_from_jump, integrate_trajectory_fixed_step, wrap_signed_angle,
};

///Original initializer82F826B0/822F8B40. This COM trajectory acceleration is
///distinct from the world's separately supplied rigid-body gravity setting.
const COM_ACCELERATION: [f32; 4] = [0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0];

pub(super) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let frame = input::frame(skater);
    let life = &mut skater.ground_lifecycle;
    life.skeleton_controller.flag_18 = true;
    life.skeleton_controller
        .request(7, &mut skater.skeleton_collision)?;
    skater.foot_ik.enable_feet(true);
    life.skeleton_controller
        .request(6, &mut skater.skeleton_collision)?;
    physics
        .board
        .hook_mut()
        .drive
        .enable_angular_only(&mut life.board_animated_290);
    skater.footplant.enabled = false;
    skater.footplant.reset();
    let state = &mut skater.air_state;
    state.selector_latch_174 = false;
    state.reached_apex = false;
    state.trajectory_query_countdown = 0;
    state.landing_normal = [0.0, 1.0, 0.0, 0.0];
    state.time_in_state = 0.0;
    state.max_y = physics.board.part_transforms()[6].translation.y;
    state.start_y = frame.ground_position_y_500;
    let (use_com, heading) = if frame.previous_physics_category_2516 == 400
        || frame.previous_physics_state_2504 == 701
    {
        (frame.flags_2468 & 0x4000 == 0, false)
    } else {
        let selected = !matches!(frame.previous_physics_state_2504, 100 | 103 | 201);
        (selected, selected)
    };
    state.use_centre_of_mass_velocity = use_com;
    if use_com {
        let mut velocity = frame.trajectory_velocity_608;
        if frame.previous_physics_state_2504 == 500 {
            velocity[1] = AirMath.minimum_vminfp(velocity[1] * f32::from_bits(0x3f26_6666), 3.0);
        }
        if frame.frames_since_jump_correction_2576 < 12 {
            velocity = calculate_velocity_from_jump(
                &frame,
                frame.frames_since_jump_correction_2576,
                &mut AirMath,
            );
        }
        state.centre_of_mass_trajectory = AirTrajectory {
            position: frame.trajectory_position_592,
            velocity,
            acceleration: COM_ACCELERATION,
            scalar_48: -1.0,
        };
    }
    if heading {
        skater.animated_skeleton.roots.initialize_heading = true;
    }
    trace_air("enter", physics, skater, &frame);
    Ok(())
}

/// SKATE2_TRACE_SKATE3_AIR: the air's centre-of-mass trajectory seeds against the deck.
fn trace_air(event: &str, physics: &Skate3PhysicsState, skater: &Skate3ControllerState, frame: &skate3_core::air::state::PhysicsAirFrame) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_AIR").is_some()) {
        return;
    }
    let deck = physics.board.bodies()[skate3_core::physics::board::BodyId::Deck.index()].rates.linear_velocity;
    let physical = &skater.player_input.physical;
    let raw = |v: [u32; 4]| v.map(f32::from_bits);
    eprintln!(
        "skate3 air: {event}: previous_state={} use_com={} seed_position_592={:?} seed_velocity_608={:?} air_reckoning_452={} air_velocity_160={:?} com_velocity_16={:?} current_velocity_400={:?} deck_velocity={:?} frames_since_jump={}",
        frame.previous_physics_state_2504, skater.air_state.use_centre_of_mass_velocity,
        frame.trajectory_position_592, frame.trajectory_velocity_608,
        physical.air.use_air_reckoning_452, raw(physical.air.vector_160), raw(physical.reckoning.vector_16),
        frame.current_velocity_400, [deck.x, deck.y, deck.z], frame.frames_since_jump_correction_2576,
    );
}

pub(super) fn exit(skater: &mut Skate3ControllerState) {
    skater.air_state.use_centre_of_mass_velocity = false;
    skater.air_reckoning.state.reset_spin();
}

pub(super) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let frame = input::frame(skater);
    skater.ground_lifecycle.skeleton_controller.update_air(
        skater.player_input.processed.flags_2472,
        frame.current_velocity_400[1],
        &mut skater.skeleton_collision,
    )?;
    let state = &mut skater.air_state;
    state.trajectory_query_countdown = state.trajectory_query_countdown.wrapping_sub(1);
    if state.trajectory_query_countdown <= 0
        && frame.flags_2468 & 0x8000 == 0
        && frame.previous_physics_state_2504 != 0
        && !skater.trajectory.selector.pending()
    {
        let mut info = input::launch_info(physics, skater)?;
        if frame.frames_since_jump_correction_2576 < 12 {
            info.start_velocity = calculate_velocity_from_jump(
                &frame,
                frame.frames_since_jump_correction_2576,
                &mut AirMath,
            );
        } else if skater.air_state.use_centre_of_mass_velocity {
            //COM helper82D34570 repeats Fill, then offsets start/board and
            //narrows the cone. Caller clears player_jumped again before Launch.
            let velocity = skater.air_state.centre_of_mass_trajectory.velocity;
            info.start_velocity = std::array::from_fn(|i| {
                COM_ACCELERATION[i].mul_add(f32::from_bits(0x3c88_8889), velocity[i])
            });
            info.start_position_override = info.animation_com_position;
            info.start_position_override[1] += f32::from_bits(0xbf26_6666);
            info.board_position_override = info.animation_com_position;
            info.board_position_override[1] += f32::from_bits(0xbf4c_cccd);
            info.use_position_override = true;
            info.trajectory_count = 5;
            info.cone_angle_x *= f32::from_bits(0x3ecc_cccd);
            info.cone_angle_z *= f32::from_bits(0x3ecc_cccd);
        }
        info.player_jumped = false;
        let input = input::selector_input(physics, skater)?;
        skater.trajectory.launch(info, input, &physics.world)?;
        skater.air_state.trajectory_query_countdown = 7;
    }
    let input = input::selector_input(physics, skater)?;
    let grind_context = super::air_trajectory::GrindContext::from_processed(
        &skater.player_input.processed,
        skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Air trajectory requires current board toolkit")?
            .deck[3],
    );
    skater
        .trajectory
        .update(input, &physics.world, grind_context)?;
    let current = skater
        .air_reckoning
        .fields(&physics.riding)
        .current_landing_normal_1152;
    let normal = skater
        .trajectory
        .selector
        .suggested_normal()
        .unwrap_or(current);
    let state = &mut skater.air_state;
    state.landing_normal = normal;
    let settings = &skater.air_settings.state;
    let mut blend = if wrap_signed_angle(angle_between_vectors(normal, current))
        <= settings.landing_normal_angle_limit_444
    {
        settings.landing_normal_blend_388
    } else {
        0.0
    };
    let mut latched = false;
    if !state.use_centre_of_mass_velocity {
        if !state.selector_latch_174 {
            state.selector_latch_174 = skater.trajectory.selector.all_predictions_missed();
        }
        if state.selector_latch_174 {
            state.use_centre_of_mass_velocity = true;
            state.centre_of_mass_trajectory = AirTrajectory {
                position: frame.trajectory_position_592,
                velocity: frame.trajectory_velocity_608,
                acceleration: COM_ACCELERATION,
                scalar_48: -1.0,
            };
            latched = true;
        }
    }
    if frame.state_timer_2664 <= 0.0 {
        blend = 0.0;
    } else if frame.flags_2468 & 0x4000 != 0 {
        state.use_centre_of_mass_velocity = false;
    }
    let spin =
        (frame.body_spin_input_2640 * settings.body_spin_scale_428) * f32::from_bits(0x3c8e_fa35);
    let target_spin = settings
        .body_spin_over_time_320
        .evaluate(state.time_in_state * 2.0)
        * spin;
    skater.air_reckoning.update(
        &mut physics.riding,
        &skater.player_input.processed,
        skater.animation_input.extra.physical_body_spin,
        normal,
        blend,
        target_spin,
        0.0,
    )?;
    if latched {
        trace_air("all predictions missed: centre of mass trajectory", physics, skater, &frame);
    }
    if skater.air_state.use_centre_of_mass_velocity {
        integrate_trajectory_fixed_step(&mut skater.air_state.centre_of_mass_trajectory);
        skeleton::advance(
            physics,
            skater,
            Some(skater.air_state.centre_of_mass_trajectory.position),
        )?;
    } else {
        skeleton::advance(physics, skater, None)?;
    }
    board::advance(physics, skater, frame)?;
    skater.skeleton_output.correction.pending = true;
    let height = physics.board.part_transforms()[6].translation.y;
    let state = &mut skater.air_state;
    if !(state.max_y > height) {
        state.max_y = height;
    }
    state.time_in_state += frame.delta_time_2604;
    Ok(())
}

///82D34DD8. The common post-physics coordinator immediately follows this
///apex latch with Wipeout::CheckForAirWipeout(false), after collision feedback.
pub(super) fn update_apex(physics: &Skate3PhysicsState, state: &mut PhysicsAirState) {
    if !state.reached_apex && physics.board.bodies()[6].rates.linear_velocity.y < 0.0 {
        state.reached_apex = true;
    }
}
